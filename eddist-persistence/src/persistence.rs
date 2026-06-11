use std::env;

use eddist_core::{domain::pubsub_repository::CreatingRes, redis_keys::DB_FAILED_CACHE_RES_KEY};
use redis::AsyncCommands;
use sqlx::{Connection, Executor, QueryBuilder, query};
use tokio::{select, time::sleep};
use tracing::{error, info};

pub async fn run_persistence_loop(
    mut conn: redis::aio::ConnectionManager,
    mut ctrl_c_rx: tokio::sync::broadcast::Receiver<()>,
) {
    let redis_url = env::var("REDIS_URL").unwrap();
    let database_url = env::var("DATABASE_URL").unwrap();
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

        let db_conn = sqlx::MySqlConnection::connect(&database_url).await;
        let db_conn = match db_conn {
            Ok(mut db_conn) => {
                // Set TIME_TRUNCATE_FRACTIONAL mode to match chrono truncation behavior
                if let Err(e) = db_conn
                    .execute(
                        "SET SESSION sql_mode = CONCAT(@@sql_mode, ',TIME_TRUNCATE_FRACTIONAL')",
                    )
                    .await
                {
                    error!(
                        error = e.to_string().as_str(),
                        "failed to set TIME_TRUNCATE_FRACTIONAL mode"
                    );
                }
                Some(db_conn)
            }
            Err(sqlx::Error::Io(e)) => {
                error!(error = e.to_string().as_str(), "failed to connect to db");
                None
            }
            Err(sqlx::Error::Tls(e)) => {
                error!(error = e.to_string().as_str(), "failed to connect to db");
                None
            }
            Err(_) => panic!(),
        };

        let mut db_conn = match db_conn {
            Some(db_conn) => db_conn,
            None => continue,
        };

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

        if let Err(e) = insert_multiple_res(&mut db_conn, &res_list).await {
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

async fn insert_multiple_res(
    conn: &mut sqlx::MySqlConnection,
    res_list: &[CreatingRes],
) -> Result<(), sqlx::Error> {
    let mut tx = conn.begin().await?;
    // bulk insert (max 1000)
    for chunk in res_list.chunks(1000) {
        // HashMap<thread_id, most recent created_at in same thread_id>
        let mut thread_id_to_created_at = std::collections::HashMap::new();
        for res in chunk {
            let created_at = thread_id_to_created_at
                .entry(res.thread_id)
                .or_insert(res.created_at);
            if res.created_at > *created_at {
                *created_at = res.created_at;
            }
        }

        let mut builder = QueryBuilder::new(
            "INSERT IGNORE INTO responses (
                    id,
                    author_name,
                    mail,
                    author_id,
                    body,
                    thread_id,
                    board_id,
                    ip_addr,
                    authed_token_id,
                    created_at,
                    client_info,
                    res_order
                )",
        );

        builder.push_values(chunk, |mut b, res| {
            let client_info = serde_json::to_string(&res.client_info).unwrap();

            b.push_bind(res.id)
                .push_bind(&res.name)
                .push_bind(&res.mail)
                .push_bind(&res.author_ch5id)
                .push_bind(&res.body)
                .push_bind(res.thread_id)
                .push_bind(res.board_id)
                .push_bind(&res.ip_addr)
                .push_bind(res.authed_token_id)
                .push_bind(res.created_at)
                .push_bind(client_info)
                .push_bind(res.res_order);
        });

        let query = builder.build();

        if let Err(e) = query.execute(&mut *tx).await {
            if matches!(&e, sqlx::Error::Database(de) if de.kind() == sqlx::error::ErrorKind::ForeignKeyViolation)
            {
                // One row in the chunk references a thread/board that no longer exists
                // (e.g. the thread was archive-deleted before this response could be
                // replayed). A multi-row INSERT is atomic, so that single poisoned row
                // would otherwise abort the whole chunk and retry forever. Fall back to
                // inserting rows one at a time, dropping only the poisoned ones.
                for res in chunk {
                    if let Err(e) = insert_single_res(&mut tx, res).await {
                        if matches!(&e, sqlx::Error::Database(de) if de.kind() == sqlx::error::ErrorKind::ForeignKeyViolation)
                        {
                            error!(
                                response_id = ?res.id,
                                thread_id = ?res.thread_id,
                                "Dropping cached response referencing a deleted thread/board"
                            );
                        } else {
                            return Err(e);
                        }
                    }
                }
            } else {
                return Err(e);
            }
        }

        for (thread_id, created_at) in thread_id_to_created_at.iter() {
            // query which is updating to responses_count, last_modified_at and active
            // response_count is calculated by select count(*) from responses where thread_id = ?
            // active is calculated response_count <= 1000, unless the thread was already
            // archived (in which case it must stay inactive). last_modified_at only moves
            // forward, so it can't be rewound by replaying older cached responses.
            // NOTE: this query is not crusial, so we can ignore the error
            let query = query!(
                r#"
            WITH response_count AS (
                SELECT COUNT(*) AS cnt
                FROM responses
                WHERE thread_id = ?
            ) UPDATE threads
            SET response_count = (SELECT cnt FROM response_count),
                last_modified_at = GREATEST(last_modified_at, ?),
                active = CASE
                    WHEN archived = 1 THEN 0
                    ELSE (SELECT cnt FROM response_count) <= 1000
                END
            WHERE id = ?;
        "#,
                thread_id,
                created_at,
                thread_id
            );
            let _ = query.execute(&mut *tx).await;
        }
    }

    tx.commit().await?;
    Ok(())
}

async fn insert_single_res(
    tx: &mut sqlx::Transaction<'_, sqlx::MySql>,
    res: &CreatingRes,
) -> Result<(), sqlx::Error> {
    let client_info = serde_json::to_string(&res.client_info).unwrap();

    query!(
        r#"
        INSERT IGNORE INTO responses (
            id,
            author_name,
            mail,
            author_id,
            body,
            thread_id,
            board_id,
            ip_addr,
            authed_token_id,
            created_at,
            client_info,
            res_order
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        res.id,
        res.name,
        res.mail,
        res.author_ch5id,
        res.body,
        res.thread_id,
        res.board_id,
        res.ip_addr,
        res.authed_token_id,
        res.created_at,
        client_info,
        res.res_order,
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}
