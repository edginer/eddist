use std::{convert::Infallible, env};

use eddist_core::{
    domain::pubsub_repository::{CreatingRes, PubSubItem},
    tracing::init_tracing,
    utils::is_prod,
};
use futures::StreamExt;
use hyper::{server::conn::http1, service::service_fn, Response};
use hyper_util::rt::{TokioIo, TokioTimer};
use redis::AsyncCommands;
use sqlx::{query, Connection, QueryBuilder};
use tokio::net::TcpListener;
use tokio::{join, select, time::sleep};
use tracing::{error_span, info_span};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if !is_prod() {
        dotenvy::dotenv().ok();
    }

    init_tracing();

    let (ctrl_c_tx, _) = tokio::sync::broadcast::channel::<()>(1);
    let ctrl_c_sub_persitence = ctrl_c_tx.subscribe();
    let ctrl_c_sub_sub = ctrl_c_tx.subscribe();

    tokio::spawn(async move {
        let listener = TcpListener::bind("0.0.0.0:9874").await.unwrap();
        if let Ok((stream, _)) = listener.accept().await {
            let svc = service_fn(|_| async move {
                let response = Response::new("Request received. Shutting down.\n".to_string());

                Ok::<_, Infallible>(response)
            });

            let mut builder = http1::Builder::new();
            let builder = builder.timer(TokioTimer::new());
            builder
                .serve_connection(TokioIo::new(stream), svc)
                .await
                .unwrap();
        }

        error_span!(
            "received shutdown signal",
            message = "shutting down the service"
        );

        ctrl_c_tx.send(()).unwrap();
    });

    let client = redis::Client::open(env::var("REDIS_URL").unwrap())?;
    let pubsub_conn = client.get_async_pubsub().await?;
    let conn = client.get_connection_manager().await?;

    let mut sub_repo = RedisSubRepository {
        pubsub_conn,
        conn: conn.clone(),
        cancel: ctrl_c_sub_sub,
    };

    let subscribe_handle = tokio::spawn(async move { sub_repo.subscribe().await });
    let persistence_handle = tokio::spawn(async move {
        let mut ctrl_c_rx = ctrl_c_sub_persitence;
        let mut conn = conn;
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
                error_span!("Redis connection lost, attempting to reconnect");
                match redis::Client::open(redis_url.clone()) {
                    Ok(client) => match client.get_connection_manager().await {
                        Ok(new_conn) => {
                            conn = new_conn;
                            is_redis_connected = true;
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

            let db_conn = sqlx::MySqlConnection::connect(&env::var("DATABASE_URL").unwrap()).await;
            let db_conn = match db_conn {
                Ok(mut db_conn) => {
                    // Set TIME_TRUNCATE_FRACTIONAL mode to match chrono truncation behavior
                    use sqlx::Executor;
                    if let Err(e) = db_conn.execute("SET SESSION sql_mode = CONCAT(@@sql_mode, ',TIME_TRUNCATE_FRACTIONAL')").await {
                        error_span!("failed to set TIME_TRUNCATE_FRACTIONAL mode", error = e.to_string().as_str());
                    }
                    Some(db_conn)
                }
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

            let res_list_result = conn
                .lrange::<'_, _, Vec<String>>("bbs:db_failed_cache:res", 0, -1)
                .await;

            let res_list = match res_list_result {
                Ok(list) => {
                    // Reset error count on successful Redis operation
                    redis_error_count = 0;
                    is_redis_connected = true;
                    list
                }
                Err(e) => {
                    error_span!("Failed to read from Redis", error = e.to_string().as_str());
                    is_redis_connected = false;
                    redis_error_count = redis_error_count.saturating_add(1);

                    // Apply exponential backoff to avoid tight loop
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

            // remove all res from cache
            if let Err(e) = conn.del::<'_, _, ()>("bbs:db_failed_cache:res").await {
                error_span!(
                    "Failed to clear Redis cache",
                    error = e.to_string().as_str()
                );
                is_redis_connected = false;
            }
        }
    });

    match join!(subscribe_handle, persistence_handle) {
        (Ok(_), Ok(_)) => {}
        _ => panic!(),
    }

    Ok(())
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

struct RedisSubRepository {
    pubsub_conn: redis::aio::PubSub,
    conn: redis::aio::ConnectionManager,
    cancel: tokio::sync::broadcast::Receiver<()>,
}

trait SubRepository {
    async fn subscribe(&mut self) -> Result<(), anyhow::Error>;
}

impl SubRepository for RedisSubRepository {
    async fn subscribe(&mut self) -> Result<(), anyhow::Error> {
        let mut error_count = 0u32;
        let redis_url = env::var("REDIS_URL").unwrap();

        loop {
            // (Re)subscribe to the channel
            if let Err(e) = self.pubsub_conn.subscribe("bbs:pubsubitem").await {
                error_span!(
                    "Failed to subscribe to Redis pubsub",
                    error = e.to_string().as_str()
                );

                // Apply exponential backoff
                let backoff_secs = std::cmp::min(2u64.pow(error_count), 60);
                error_span!(
                    "Backing off for {} seconds before retry",
                    seconds = backoff_secs
                );
                sleep(std::time::Duration::from_secs(backoff_secs)).await;
                error_count = error_count.saturating_add(1);

                // Attempt to reconnect
                match redis::Client::open(redis_url.clone()) {
                    Ok(client) => match client.get_async_pubsub().await {
                        Ok(new_pubsub) => {
                            self.pubsub_conn = new_pubsub;
                            info_span!("Successfully reconnected to Redis pubsub");
                            continue;
                        }
                        Err(e) => {
                            error_span!(
                                "Failed to get pubsub connection",
                                error = e.to_string().as_str()
                            );
                            continue;
                        }
                    },
                    Err(e) => {
                        error_span!(
                            "Failed to create Redis client",
                            error = e.to_string().as_str()
                        );
                        continue;
                    }
                }
            }

            log::info!("Application starts subscribing to pubsub channel");
            error_count = 0; // Reset error count on successful subscription

            let subscribe_result = self.handle_messages().await;

            match subscribe_result {
                Ok(true) => {
                    // Normal shutdown requested
                    break;
                }
                Ok(false) => {
                    // Connection lost, will retry
                    error_span!("Redis pubsub connection lost, attempting to reconnect");
                    error_count = error_count.saturating_add(1);

                    // Apply exponential backoff before reconnecting
                    let backoff_secs = std::cmp::min(2u64.pow(error_count), 60);
                    sleep(std::time::Duration::from_secs(backoff_secs)).await;

                    // Recreate pubsub connection
                    match redis::Client::open(redis_url.clone()) {
                        Ok(client) => match client.get_async_pubsub().await {
                            Ok(new_pubsub) => {
                                self.pubsub_conn = new_pubsub;
                                info_span!("Successfully reconnected to Redis pubsub");
                            }
                            Err(e) => {
                                error_span!(
                                    "Failed to get pubsub connection",
                                    error = e.to_string().as_str()
                                );
                            }
                        },
                        Err(e) => {
                            error_span!(
                                "Failed to create Redis client",
                                error = e.to_string().as_str()
                            );
                        }
                    }
                }
                Err(e) => {
                    error_span!("Error in message handling", error = e.to_string().as_str());
                    error_count = error_count.saturating_add(1);

                    let backoff_secs = std::cmp::min(2u64.pow(error_count), 60);
                    sleep(std::time::Duration::from_secs(backoff_secs)).await;
                }
            }
        }

        let _ = self.pubsub_conn.unsubscribe("bbs:pubsubitem").await;

        Ok(())
    }
}

impl RedisSubRepository {
    /// Handle incoming messages. Returns Ok(true) for shutdown, Ok(false) for connection lost.
    async fn handle_messages(&mut self) -> Result<bool, anyhow::Error> {
        loop {
            let mut on_message = self.pubsub_conn.on_message();
            let msg = select! {
                _ = self.cancel.recv() => {
                    return Ok(true); // Shutdown requested
                }
                msg = on_message.next() => msg,
            };

            let Some(msg) = msg else {
                // Stream ended, connection likely lost
                error_span!("Pubsub message stream ended");
                return Ok(false);
            };

            info_span!(
                "received pubsub message",
                payload = msg.get_payload::<String>().unwrap_or_default().as_str()
            );

            let payload = match msg.get_payload::<String>() {
                Ok(p) => p,
                Err(e) => {
                    error_span!(
                        "Failed to get message payload",
                        error = e.to_string().as_str()
                    );
                    continue;
                }
            };

            let item = match serde_json::from_str::<PubSubItem>(&payload) {
                Ok(i) => i,
                Err(e) => {
                    error_span!(
                        "Failed to parse pubsub item",
                        error = e.to_string().as_str()
                    );
                    continue;
                }
            };

            match item {
                PubSubItem::CreatingRes(res) => {
                    let mut conn = self.conn.clone();
                    let res = serde_json::to_string(&res)?;

                    if let Err(e) = conn
                        .rpush::<'_, _, _, ()>("bbs:db_failed_cache:res", res)
                        .await
                    {
                        error_span!(
                            "Failed to push to Redis cache",
                            error = e.to_string().as_str()
                        );
                        return Ok(false); // Connection likely lost
                    }
                }
                PubSubItem::Shutdown => {
                    return Ok(true); // Shutdown requested
                }
            }
        }
    }
}
