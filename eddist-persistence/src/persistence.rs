use std::{collections::HashMap, env, ops::Deref};

use chrono::{DateTime, Utc};
use eddist_core::{domain::pubsub_repository::CreatingRes, redis_keys::DB_FAILED_CACHE_RES_KEY};
use eddist_entity::{db_time::truncate_to_millis, response, thread};
use redis::AsyncCommands;
use sea_orm::{
    ActiveValue::Set,
    ColumnTrait, DatabaseConnection, DatabaseTransaction, DbErr, EntityTrait, QueryFilter,
    RuntimeErr, SqlErr, TransactionTrait,
    sea_query::{Expr, ExprTrait, Func, OnConflict, Query},
};
use tokio::{select, time::sleep};
use tracing::{error, info, warn};

const MAX_ACTIVE_RESPONSE_COUNT: i64 = 1000;

pub async fn run_persistence_loop(
    mut conn: redis::aio::ConnectionManager,
    db: DatabaseConnection,
    mut ctrl_c_rx: tokio::sync::broadcast::Receiver<()>,
) {
    let redis_url = env::var("REDIS_URL").unwrap();
    let mut redis_error_count = 0u32;
    let mut is_redis_connected = true;

    loop {
        select! {
            _ = sleep(std::time::Duration::from_secs(10)) => {}
            _ = ctrl_c_rx.recv() => {
                break;
            }
        };

        // Check Redis connection health and attempt reconnection if needed
        if !is_redis_connected {
            error!("Redis connection lost, attempting to reconnect");
            match redis::Client::open(redis_url.clone()) {
                Ok(client) => match client.get_connection_manager().await {
                    Ok(new_conn) => {
                        conn = new_conn;
                        redis_error_count = 0;
                        info!("Successfully reconnected to Redis");
                    }
                    Err(e) => {
                        error!(
                            error = e.to_string().as_str(),
                            "Failed to reconnect to Redis"
                        );
                        let backoff_secs = std::cmp::min(2u64.pow(redis_error_count), 60);
                        sleep(std::time::Duration::from_secs(backoff_secs)).await;
                        redis_error_count = redis_error_count.saturating_add(1);
                        continue;
                    }
                },
                Err(e) => {
                    error!(
                        error = e.to_string().as_str(),
                        "Failed to create Redis client"
                    );
                    let backoff_secs = std::cmp::min(2u64.pow(redis_error_count), 60);
                    sleep(std::time::Duration::from_secs(backoff_secs)).await;
                    redis_error_count = redis_error_count.saturating_add(1);
                    continue;
                }
            }
        }

        let res_list_result = conn
            .lrange::<'_, _, Vec<String>>(DB_FAILED_CACHE_RES_KEY, 0, -1)
            .await;

        let res_list = match res_list_result {
            Ok(list) => {
                redis_error_count = 0;
                is_redis_connected = true;
                list
            }
            Err(e) => {
                error!(error = e.to_string().as_str(), "Failed to read from Redis");
                is_redis_connected = false;
                redis_error_count = redis_error_count.saturating_add(1);

                let backoff_secs = std::cmp::min(2u64.pow(redis_error_count), 60);
                error!("Backing off for {backoff_secs} seconds before retry");
                sleep(std::time::Duration::from_secs(backoff_secs)).await;
                continue;
            }
        };

        if res_list.is_empty() {
            continue;
        }

        let res_count = res_list.len();
        let res_list = res_list
            .iter()
            .filter_map(|res| match serde_json::from_str::<CreatingRes>(res) {
                Ok(res) => Some(res),
                Err(e) => {
                    error!(
                        error = e.to_string().as_str(),
                        "Failed to parse cached response, dropping it"
                    );
                    None
                }
            })
            .collect::<Vec<_>>();

        if let Err(e) = insert_multiple_res(&db, &res_list).await {
            error!(
                error = e.to_string().as_str(),
                "Failed to insert responses to DB"
            );
            continue;
        }

        // Remove only the entries we just read; entries pushed concurrently
        // (which now sit after index `res_count - 1`) are preserved.
        if let Err(e) = conn
            .ltrim::<'_, _, ()>(DB_FAILED_CACHE_RES_KEY, res_count as isize, -1)
            .await
        {
            error!(error = e.to_string().as_str(), "Failed to trim Redis cache");
            is_redis_connected = false;
        }
    }
}

pub(crate) async fn insert_multiple_res(
    db: &DatabaseConnection,
    res_list: &[CreatingRes],
) -> Result<(), DbErr> {
    let tx = db.begin().await?;
    for chunk in res_list.chunks(1000) {
        let mut thread_id_to_created_at = HashMap::new();
        for res in chunk {
            let created_at = truncate_to_millis(res.created_at);
            let latest = thread_id_to_created_at
                .entry(res.thread_id)
                .or_insert(created_at);
            if created_at > *latest {
                *latest = created_at;
            }
        }

        if let Err(e) = insert_res_in_savepoint(&tx, chunk).await {
            if !is_row_error(&e) {
                return Err(e);
            }
            // A multi-row INSERT is atomic, so one rejected row would otherwise
            // fail the whole chunk on every retry.
            for res in chunk {
                let Err(e) = insert_res_in_savepoint(&tx, std::slice::from_ref(res)).await else {
                    continue;
                };
                if !is_row_error(&e) {
                    return Err(e);
                }
                if matches!(e.sql_err(), Some(SqlErr::ForeignKeyConstraintViolation(_))) {
                    error!(
                        response_id = ?res.id,
                        thread_id = ?res.thread_id,
                        "Dropping cached response referencing a deleted thread/board"
                    );
                } else {
                    error!(
                        error = e.to_string().as_str(),
                        response_id = ?res.id,
                        thread_id = ?res.thread_id,
                        "Dropping cached response rejected by the database"
                    );
                }
            }
        }

        // Not crucial, so failures are only logged.
        for (thread_id, created_at) in thread_id_to_created_at {
            if let Err(e) = update_thread_stats_in_savepoint(&tx, thread_id, created_at).await {
                warn!(
                    error = e.to_string().as_str(),
                    thread_id = ?thread_id,
                    "Failed to update thread stats"
                );
            }
        }
    }

    tx.commit().await
}

// PostgreSQL aborts the whole transaction on any failed statement, so every
// statement whose failure is tolerated must run in its own savepoint.
async fn insert_res_in_savepoint(
    tx: &DatabaseTransaction,
    res_list: &[CreatingRes],
) -> Result<(), DbErr> {
    let savepoint = tx.begin().await?;
    let result = response::Entity::insert_many(res_list.iter().map(to_active_model))
        .on_conflict(
            OnConflict::column(response::Column::Id)
                .do_nothing_on([response::Column::Id])
                .to_owned(),
        )
        .exec_without_returning(&savepoint)
        .await;
    match result {
        Ok(_) => savepoint.commit().await,
        Err(e) => {
            savepoint.rollback().await?;
            Err(e)
        }
    }
}

async fn update_thread_stats_in_savepoint(
    tx: &DatabaseTransaction,
    thread_id: uuid::Uuid,
    created_at: DateTime<Utc>,
) -> Result<(), DbErr> {
    let savepoint = tx.begin().await?;
    let result = update_thread_stats(&savepoint, thread_id, created_at).await;
    match result {
        Ok(()) => savepoint.commit().await,
        Err(e) => {
            savepoint.rollback().await?;
            Err(e)
        }
    }
}

// The count subquery is repeated instead of reading `response_count` back:
// MySQL evaluates SET left to right with updated values, PostgreSQL does not.
// `last_modified_at` only moves forward so replaying older responses can't
// rewind it, and archived threads must stay inactive.
async fn update_thread_stats(
    db: &DatabaseTransaction,
    thread_id: uuid::Uuid,
    created_at: DateTime<Utc>,
) -> Result<(), DbErr> {
    let response_count = || {
        Query::select()
            .expr(Expr::col(response::Column::Id).count())
            .from(response::Entity)
            .and_where(response::Column::ThreadId.eq(thread_id))
            .to_owned()
    };

    thread::Entity::update_many()
        .col_expr(thread::Column::ResponseCount, response_count().into())
        .col_expr(
            thread::Column::LastModifiedAt,
            Func::greatest([
                Expr::col(thread::Column::LastModifiedAt),
                Expr::value(created_at),
            ])
            .into(),
        )
        .col_expr(
            thread::Column::Active,
            Expr::case(Expr::col(thread::Column::Archived), false)
                .finally(Expr::expr(response_count()).lte(MAX_ACTIVE_RESPONSE_COUNT))
                .into(),
        )
        .filter(thread::Column::Id.eq(thread_id))
        .exec(db)
        .await?;
    Ok(())
}

fn to_active_model(res: &CreatingRes) -> response::ActiveModel {
    response::ActiveModel {
        id: Set(res.id),
        author_name: Set(res.name.clone()),
        mail: Set(res.mail.clone()),
        body: Set(res.body.clone()),
        created_at: Set(truncate_to_millis(res.created_at)),
        author_id: Set(res.author_ch5id.clone()),
        ip_addr: Set(res.ip_addr.clone()),
        authed_token_id: Set(res.authed_token_id),
        board_id: Set(res.board_id),
        thread_id: Set(res.thread_id),
        res_order: Set(res.res_order),
        client_info: Set(serde_json::to_value(&res.client_info).unwrap()),
        ..Default::default()
    }
}

// SQLSTATE class 22 (data exception) and 23 (integrity constraint violation)
// mean the row itself is bad on both MySQL and PostgreSQL; anything else
// (connection loss, a broken statement) must not drop data.
fn is_row_error(e: &DbErr) -> bool {
    let (DbErr::Exec(RuntimeErr::SqlxError(err)) | DbErr::Query(RuntimeErr::SqlxError(err))) = e
    else {
        return false;
    };
    let sea_orm::sqlx::Error::Database(db_err) = err.deref() else {
        return false;
    };
    db_err
        .code()
        .is_some_and(|code| code.starts_with("22") || code.starts_with("23"))
}
