use std::env;

use eddist_core::{domain::pubsub_repository::CreatingRes, redis_keys::DB_FAILED_CACHE_RES_KEY};
use redis::AsyncCommands;
use sqlx::{Connection, Executor, QueryBuilder, query};
use tokio::{select, time::sleep};
use tracing::{error_span, info_span};

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
            error_span!("Redis connection lost, attempting to reconnect");
            match redis::Client::open(redis_url.clone()) {
                Ok(client) => match client.get_connection_manager().await {
                    Ok(new_conn) => {
                        conn = new_conn;
                        redis_error_count = 0;
                        info_span!("Successfully reconnected to Redis");
                    }
                    Err(e) => {
                        error_span!(
                            "Failed to reconnect to Redis",
                            error = e.to_string().as_str()
                        );
                        let backoff_secs = std::cmp::min(2u64.pow(redis_error_count), 60);
                        sleep(std::time::Duration::from_secs(backoff_secs)).await;
                        redis_error_count = redis_error_count.saturating_add(1);
                        continue;
                    }
                },
                Err(e) => {
                    error_span!(
                        "Failed to create Redis client",
                        error = e.to_string().as_str()
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
                error_span!("Failed to read from Redis", error = e.to_string().as_str());
                is_redis_connected = false;
                redis_error_count = redis_error_count.saturating_add(1);

                let backoff_secs = std::cmp::min(2u64.pow(redis_error_count), 60);
                error_span!(
                    "Backing off for {} seconds before retry",
                    seconds = backoff_secs
                );
                sleep(std::time::Duration::from_secs(backoff_secs)).await;
                continue;
            }
        };

        if res_list.is_empty() {
            continue;
        }

        #[cfg(not(feature = "backend-postgres"))]
        let db_conn = sqlx::MySqlConnection::connect(&database_url).await;
        #[cfg(feature = "backend-postgres")]
        let db_conn = sqlx::PgConnection::connect(&database_url).await;

        let db_conn = match db_conn {
            // PostgreSQL: TIMESTAMPTZ has native fractional precision; no session mode needed.
            Ok(db_conn) => Some(db_conn),
            Err(sqlx::Error::Io(e)) => {
                error_span!("failed to connect to db", error = e.to_string().as_str());
                None
            }
            Err(sqlx::Error::Tls(e)) => {
                error_span!("failed to connect to db", error = e.to_string().as_str());
                None
            }
            Err(_) => panic!(),
        };

        let mut db_conn = match db_conn {
            Some(db_conn) => db_conn,
            None => continue,
        };

        let res_list = res_list
            .iter()
            .map(|res| serde_json::from_str::<CreatingRes>(res))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        if let Err(e) = insert_multiple_res(&mut db_conn, &res_list).await {
            error_span!(
                "Failed to insert responses to DB",
                error = e.to_string().as_str()
            );
            continue;
        }

        if let Err(e) = conn.del::<'_, _, ()>(DB_FAILED_CACHE_RES_KEY).await {
            error_span!(
                "Failed to clear Redis cache",
                error = e.to_string().as_str()
            );
            is_redis_connected = false;
        }
    }
}

#[cfg(not(feature = "backend-postgres"))]
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
            "INSERT INTO responses (
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
            match e {
                sqlx::Error::Database(ref database_error) => match database_error.kind() {
                    sqlx::error::ErrorKind::UniqueViolation => {}
                    _ => return Err(e),
                },
                _ => return Err(e),
            }
        };

        for (thread_id, created_at) in thread_id_to_created_at.iter() {
            // query which is updating to responses_count, last_modified_at and active
            // response_count is calculated by select count(*) from responses where thread_id = ?
            // active is calculated response_count <= 1000
            // NOTE: this query is not crusial, so we can ignore the error
            let query = query!(
                r#"
            WITH response_count AS (
                SELECT COUNT(*) AS cnt
                FROM responses
                WHERE thread_id = ?
            ) UPDATE threads
            SET response_count = (SELECT cnt FROM response_count),
                last_modified_at = ?,
                active = (SELECT cnt FROM response_count) <= 1000
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

#[cfg(feature = "backend-postgres")]
async fn insert_multiple_res(
    conn: &mut sqlx::PgConnection,
    res_list: &[CreatingRes],
) -> Result<(), sqlx::Error> {
    // TODO: Pass 2 — implement PG-specific bulk insert
    let _ = (conn, res_list);
    Ok(())
}
