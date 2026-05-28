use std::env;

use eddist_core::{
    domain::pubsub_repository::{
        CHANNEL_AUTH_TOKEN_REVOKED, CHANNEL_AUTH_TOKEN_SUCCEEDED, CHANNEL_PUBSUB_ITEM, PubSubItem,
    },
    proto::{decode_auth_token_revoked, decode_auth_token_succeeded, decode_creating_thread},
    redis_keys::{CHANNEL_THREAD_CREATED, DB_FAILED_CACHE_RES_KEY, unsafe_threads_key},
};
use futures::StreamExt;
use redis::AsyncCommands;
use tokio::{select, time::sleep};
use tracing::{error, info, warn};

use crate::token_backup::{backup_token, remove_token_backup};

pub struct RedisSubRepository {
    pubsub_conn: redis::aio::PubSub,
    conn: redis::aio::ConnectionManager,
    cancel: tokio::sync::broadcast::Receiver<()>,
    s3_bucket: Option<(aws_sdk_s3::Client, String)>,
    db_pool: Option<sqlx::MySqlPool>,
}

impl RedisSubRepository {
    pub fn new(
        pubsub_conn: redis::aio::PubSub,
        conn: redis::aio::ConnectionManager,
        cancel: tokio::sync::broadcast::Receiver<()>,
        s3_client: Option<aws_sdk_s3::Client>,
        s3_bucket_name: Option<String>,
        db_pool: Option<sqlx::MySqlPool>,
    ) -> Self {
        Self {
            pubsub_conn,
            conn,
            cancel,
            s3_bucket: s3_client.zip(s3_bucket_name),
            db_pool,
        }
    }
}

pub trait SubRepository {
    async fn subscribe(&mut self) -> Result<(), anyhow::Error>;
}

async fn reconnect_pubsub(redis_url: &str) -> anyhow::Result<redis::aio::PubSub> {
    let client = redis::Client::open(redis_url)?;
    Ok(client.get_async_pubsub().await?)
}

impl SubRepository for RedisSubRepository {
    async fn subscribe(&mut self) -> Result<(), anyhow::Error> {
        let mut error_count = 0u32;
        let redis_url = env::var("REDIS_URL").unwrap();
        let backup_enabled = self.s3_bucket.is_some();
        let mut channels = vec![CHANNEL_PUBSUB_ITEM, CHANNEL_THREAD_CREATED];
        if backup_enabled {
            channels.push(CHANNEL_AUTH_TOKEN_SUCCEEDED);
            channels.push(CHANNEL_AUTH_TOKEN_REVOKED);
        }

        loop {
            let subscribe_result = self.pubsub_conn.subscribe(channels.as_slice()).await;

            if let Err(e) = subscribe_result {
                error!(
                    error = e.to_string().as_str(),
                    "Failed to subscribe to Redis pubsub"
                );

                let backoff_secs = std::cmp::min(2u64.pow(error_count), 60);
                error!(seconds = backoff_secs, "Backing off before retry");
                sleep(std::time::Duration::from_secs(backoff_secs)).await;
                error_count = error_count.saturating_add(1);

                match reconnect_pubsub(&redis_url).await {
                    Ok(new_pubsub) => {
                        self.pubsub_conn = new_pubsub;
                        info!("Successfully reconnected to Redis pubsub");
                    }
                    Err(e) => {
                        error!(
                            error = e.to_string().as_str(),
                            "Failed to reconnect to Redis pubsub"
                        );
                    }
                }
                continue;
            }

            info!("Application starts subscribing to pubsub channel");
            error_count = 0;

            let subscribe_result = self.handle_messages().await;

            match subscribe_result {
                Ok(true) => {
                    break;
                }
                Ok(false) => {
                    error!("Redis pubsub connection lost, attempting to reconnect");
                    error_count = error_count.saturating_add(1);

                    let backoff_secs = std::cmp::min(2u64.pow(error_count), 60);
                    sleep(std::time::Duration::from_secs(backoff_secs)).await;

                    match reconnect_pubsub(&redis_url).await {
                        Ok(new_pubsub) => {
                            self.pubsub_conn = new_pubsub;
                            info!("Successfully reconnected to Redis pubsub");
                        }
                        Err(e) => {
                            error!(
                                error = e.to_string().as_str(),
                                "Failed to reconnect to Redis pubsub"
                            );
                        }
                    }
                }
                Err(e) => {
                    error!(error = e.to_string().as_str(), "Error in message handling");
                    error_count = error_count.saturating_add(1);

                    let backoff_secs = std::cmp::min(2u64.pow(error_count), 60);
                    sleep(std::time::Duration::from_secs(backoff_secs)).await;
                }
            }
        }

        let _ = self.pubsub_conn.unsubscribe(channels.as_slice()).await;

        Ok(())
    }
}

impl RedisSubRepository {
    /// Returns Ok(true) for shutdown, Ok(false) for connection lost.
    async fn handle_messages(&mut self) -> Result<bool, anyhow::Error> {
        loop {
            let mut on_message = self.pubsub_conn.on_message();
            let msg = select! {
                _ = self.cancel.recv() => {
                    return Ok(true);
                }
                msg = on_message.next() => msg,
            };

            let Some(msg) = msg else {
                error!("Pubsub message stream ended");
                return Ok(false);
            };

            let channel = msg.get_channel::<String>().unwrap_or_default();

            match channel.as_str() {
                ch if ch == CHANNEL_PUBSUB_ITEM => {
                    let payload = match msg.get_payload::<String>() {
                        Ok(p) => p,
                        Err(e) => {
                            error!(
                                error = e.to_string().as_str(),
                                "Failed to get message payload"
                            );
                            continue;
                        }
                    };

                    info!(payload = payload.as_str(), "received pubsub message");

                    let item = match serde_json::from_str::<PubSubItem>(&payload) {
                        Ok(i) => i,
                        Err(e) => {
                            error!(
                                error = e.to_string().as_str(),
                                "Failed to parse pubsub item"
                            );
                            continue;
                        }
                    };

                    match item {
                        PubSubItem::CreatingRes(res) => {
                            let mut conn = self.conn.clone();
                            let res = serde_json::to_string(&res)?;

                            if let Err(e) = conn
                                .rpush::<'_, _, _, ()>(DB_FAILED_CACHE_RES_KEY, res)
                                .await
                            {
                                error!(
                                    error = e.to_string().as_str(),
                                    "Failed to push to Redis cache"
                                );
                                return Ok(false);
                            }
                        }
                        PubSubItem::Shutdown => {
                            return Ok(true);
                        }
                    }
                }
                ch if ch == CHANNEL_AUTH_TOKEN_SUCCEEDED => {
                    let payload = match msg.get_payload::<Vec<u8>>() {
                        Ok(p) => p,
                        Err(e) => {
                            error!(
                                error = e.to_string().as_str(),
                                "Failed to get message payload"
                            );
                            continue;
                        }
                    };

                    let event = match decode_auth_token_succeeded(&payload) {
                        Ok(e) => e,
                        Err(e) => {
                            error!(
                                error = e.to_string().as_str(),
                                "Failed to decode AuthTokenSucceeded"
                            );
                            continue;
                        }
                    };
                    let token_id = event.authed_token_id;
                    if let (Some(pool), Some((client, bucket_name))) =
                        (self.db_pool.as_ref(), self.s3_bucket.as_ref())
                    {
                        let pool = pool.clone();
                        let client = client.clone();
                        let bucket_name = bucket_name.clone();
                        tokio::spawn(async move {
                            if let Err(e) =
                                backup_token(&pool, &client, &bucket_name, token_id).await
                            {
                                warn!("Failed to backup token {token_id}: {e}");
                            }
                        });
                    }
                }
                ch if ch == CHANNEL_THREAD_CREATED => {
                    let payload = match msg.get_payload::<Vec<u8>>() {
                        Ok(p) => p,
                        Err(e) => {
                            error!(
                                error = e.to_string().as_str(),
                                "Failed to get thread_created payload"
                            );
                            continue;
                        }
                    };

                    let event = match decode_creating_thread(&payload) {
                        Ok(e) => e,
                        Err(e) => {
                            error!(
                                error = e.to_string().as_str(),
                                "Failed to decode CreatingThread"
                            );
                            continue;
                        }
                    };

                    if event.moderation_result.map(|m| m.flagged).unwrap_or(false) {
                        let key = unsafe_threads_key(event.board_id);
                        let mut conn = self.conn.clone();
                        if let Err(e) = conn.sadd::<_, _, ()>(&key, event.unix_time).await {
                            error!(
                                error = e.to_string().as_str(),
                                "Failed to add unsafe thread to Redis set"
                            );
                        } else {
                            info!(
                                board_id = event.board_id.to_string().as_str(),
                                unix_time = event.unix_time,
                                "Flagged thread stored in safe mode set"
                            );
                        }
                    }
                }
                ch if ch == CHANNEL_AUTH_TOKEN_REVOKED => {
                    let payload = match msg.get_payload::<Vec<u8>>() {
                        Ok(p) => p,
                        Err(e) => {
                            error!(
                                error = e.to_string().as_str(),
                                "Failed to get message payload"
                            );
                            continue;
                        }
                    };

                    let event = match decode_auth_token_revoked(&payload) {
                        Ok(e) => e,
                        Err(e) => {
                            error!(
                                error = e.to_string().as_str(),
                                "Failed to decode AuthTokenRevoked"
                            );
                            continue;
                        }
                    };
                    let token_id = event.authed_token_id;
                    if let Some((client, bucket_name)) = self.s3_bucket.as_ref() {
                        let client = client.clone();
                        let bucket_name = bucket_name.clone();
                        tokio::spawn(async move {
                            if let Err(e) =
                                remove_token_backup(&client, &bucket_name, token_id).await
                            {
                                warn!("Failed to remove token backup {token_id}: {e}");
                            }
                        });
                    }
                }
                _ => {}
            }
        }
    }
}
