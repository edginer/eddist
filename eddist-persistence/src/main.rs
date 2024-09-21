use std::{convert::Infallible, env};

use eddist_core::{
    domain::pubsub_repository::{CreatingRes, PubSubItem},
    utils::is_prod,
};
use futures::StreamExt;
use hyper::{
    body::{Body, Bytes},
    server::conn::http1,
    service::service_fn,
    Response,
};
use hyper_util::rt::{TokioIo, TokioTimer};
use redis::AsyncCommands;
use sqlx::{query, Connection, QueryBuilder};
use tokio::net::TcpListener;
use tokio::{join, select, time::sleep};
use tracing::{error_span, info_span};
use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if !is_prod() {
        dotenvy::dotenv().ok();
    }

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_span_events(FmtSpan::CLOSE)
        .with_ansi(false)
        .init();

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
        loop {
            select! {
                _ = sleep(std::time::Duration::from_secs(10)) => {}
                _ = ctrl_c_rx.recv() => {
                    break;
                }
            };
            let db_conn = sqlx::MySqlConnection::connect(&env::var("DATABASE_URL").unwrap()).await;
            let db_conn = match db_conn {
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

            let Ok(res_list) = conn
                .lrange::<'_, _, Vec<String>>("bbs:db_failed_cache:res", 0, -1)
                .await
            else {
                // logging and continue
                continue;
            };

            if res_list.is_empty() {
                continue;
            }

            let res_list = res_list
                .iter()
                .map(|res| serde_json::from_str::<CreatingRes>(res))
                .collect::<Result<Vec<_>, _>>()
                .unwrap();

            if let Err(_e) = insert_multiple_res(&mut db_conn, &res_list).await {
                // logging and continue
                continue;
            }

            // remove all res from cache
            let _ = conn.del::<'_, _, ()>("bbs:db_failed_cache:res").await;
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
        self.pubsub_conn.subscribe("bbs:pubsubitem").await?;

        loop {
            let mut on_message = self.pubsub_conn.on_message();
            let msg = select! {
                _ = self.cancel.recv() => {
                    break;
                }
                msg = on_message.next() => msg,
            };

            let Some(msg) = msg else {
                continue;
            };

            info_span!(
                "received pubsub message",
                payload = msg.get_payload::<String>().unwrap().as_str()
            );

            let payload = msg.get_payload::<String>()?;
            let item = serde_json::from_str::<PubSubItem>(&payload)?;
            match item {
                PubSubItem::CreatingRes(res) => {
                    let mut conn = self.conn.clone();
                    let res = serde_json::to_string(&res)?;
                    conn.rpush::<'_, _, _, ()>("bbs:db_failed_cache:res", res)
                        .await?;
                }
                PubSubItem::Shutdown => {
                    break;
                }
            }
        }

        self.pubsub_conn.unsubscribe("bbs:pubsubitem").await?;

        Ok(())
    }
}
