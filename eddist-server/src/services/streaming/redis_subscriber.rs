use crate::{services::streaming::manager::StreamManager, utils::redis::event_res_created_channel};
use eddist_core::domain::pubsub_repository::CreatingRes;
use futures::StreamExt;
use std::{env, sync::Arc};
use tracing::{error, info, warn};

/// Spawn a background task that subscribes to Redis pub/sub channels
/// and forwards events to the StreamManager.
pub fn spawn_redis_subscriber(manager: Arc<StreamManager>) {
    tokio::spawn(async move {
        // Create a dedicated client for pubsub from environment
        let redis_url = match env::var("REDIS_URL") {
            Ok(url) => url,
            Err(_) => {
                error!("REDIS_URL not set, cannot start Redis subscriber");
                return;
            }
        };

        let client = match redis::Client::open(redis_url) {
            Ok(c) => c,
            Err(e) => {
                error!("Failed to create Redis client for subscriber: {e}");
                return;
            }
        };

        loop {
            if let Err(e) = run_subscriber(client.clone(), manager.clone()).await {
                error!("Redis subscriber error: {e}. Reconnecting in 5 seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        }
    });
}

async fn run_subscriber(
    redis_client: redis::Client,
    manager: Arc<StreamManager>,
) -> Result<(), anyhow::Error> {
    let mut pubsub = redis_client.get_async_pubsub().await?;
    let res_created_channel = event_res_created_channel();

    pubsub.subscribe(res_created_channel).await?;

    info!("Redis subscriber listening on channels: {res_created_channel}",);

    let mut stream = pubsub.on_message();

    while let Some(msg) = stream.next().await {
        let channel = msg.get_channel_name().to_string();
        let payload = match msg.get_payload::<String>() {
            Ok(p) => p,
            Err(e) => {
                warn!("Failed to get message payload: {e}");
                continue;
            }
        };

        match channel.as_str() {
            x if x == res_created_channel => match serde_json::from_str::<CreatingRes>(&payload) {
                Ok(event) => {
                    manager.publish(event.thread_id, String::new());
                }
                Err(e) => {
                    warn!("Failed to parse CreatingRes event: {e}");
                }
            },
            _ => {
                warn!("Received message from unknown channel: {channel}");
            }
        }
    }

    Ok(())
}
