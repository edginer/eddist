use futures::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Mutex, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, error, info};

use crate::repositories::bbs_repository::BbsRepository;
use crate::utils::redis::thread_ws_updates_key;

/// Manages shared Redis PubSub connections for WebSocket clients
/// One Redis subscription per thread, broadcast to multiple clients
#[derive(Clone)]
pub struct WebSocketManager<T: BbsRepository> {
    subscriptions: Arc<RwLock<HashMap<String, Arc<ThreadSubscription>>>>,
    bbs_repo: T,
}

struct ThreadSubscription {
    broadcast_tx: broadcast::Sender<String>,
    subscriber_count: Arc<Mutex<usize>>,
    _task_handle: JoinHandle<()>,
}

impl<T: BbsRepository + Clone> WebSocketManager<T> {
    pub fn new(bbs_repo: T) -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            bbs_repo,
        }
    }

    /// Subscribe to thread updates
    /// Returns a broadcast receiver for the thread
    pub async fn subscribe(
        &self,
        board_key: String,
        thread_number: u64,
    ) -> Result<broadcast::Receiver<String>, anyhow::Error> {
        let channel_key = thread_ws_updates_key(&board_key, thread_number);

        // Check if subscription already exists
        {
            let subscriptions = self.subscriptions.read().await;
            if let Some(sub) = subscriptions.get(&channel_key) {
                let mut count = sub.subscriber_count.lock().await;
                *count += 1;
                info!(
                    "Reusing existing subscription for thread {} on board {} (subscribers: {})",
                    thread_number, board_key, *count
                );
                return Ok(sub.broadcast_tx.subscribe());
            }
        }

        // Create new subscription
        let (broadcast_tx, _) = broadcast::channel(100);
        let subscriber_count = Arc::new(Mutex::new(1));

        let channel_key_clone = channel_key.clone();
        let board_key_clone = board_key.clone();
        let broadcast_tx_clone = broadcast_tx.clone();
        let subscriber_count_clone = subscriber_count.clone();
        let subscriptions_clone = self.subscriptions.clone();
        let bbs_repo_clone = self.bbs_repo.clone();

        // Spawn Redis listener task
        let task_handle = tokio::spawn(async move {
            if let Err(e) = Self::redis_listener_task(
                channel_key_clone.clone(),
                board_key_clone,
                thread_number,
                broadcast_tx_clone,
                subscriber_count_clone,
                subscriptions_clone,
                bbs_repo_clone,
            )
            .await
            {
                error!("Redis listener task failed: {}", e);
            }
        });

        let subscription = Arc::new(ThreadSubscription {
            broadcast_tx: broadcast_tx.clone(),
            subscriber_count: subscriber_count.clone(),
            _task_handle: task_handle,
        });

        // Store subscription
        {
            let mut subscriptions = self.subscriptions.write().await;
            subscriptions.insert(channel_key.clone(), subscription);
        }

        info!(
            "Created new Redis subscription for thread {} on board {}",
            thread_number, board_key
        );

        Ok(broadcast_tx.subscribe())
    }

    /// Decrement subscriber count for a thread
    pub async fn unsubscribe(&self, board_key: &str, thread_number: u64) {
        let channel_key = thread_ws_updates_key(board_key, thread_number);

        let subscriptions = self.subscriptions.read().await;
        if let Some(sub) = subscriptions.get(&channel_key) {
            let mut count = sub.subscriber_count.lock().await;
            if *count > 0 {
                *count -= 1;
                info!(
                    "Client unsubscribed from thread {} on board {} (remaining: {})",
                    thread_number, board_key, *count
                );
            }
        }
    }

    /// Redis listener task - one per thread
    async fn redis_listener_task(
        channel_key: String,
        board_key: String,
        thread_number: u64,
        broadcast_tx: broadcast::Sender<String>,
        subscriber_count: Arc<Mutex<usize>>,
        subscriptions: Arc<RwLock<HashMap<String, Arc<ThreadSubscription>>>>,
        bbs_repo: T,
    ) -> Result<(), anyhow::Error> {
        let redis_client = redis::Client::open(std::env::var("REDIS_URL")?)?;
        let mut pubsub = redis_client.get_async_pubsub().await?;

        pubsub.subscribe(&channel_key).await?;
        info!(
            "Redis listener started for thread {} on board {}",
            thread_number, board_key
        );

        let mut message_stream = pubsub.into_on_message();
        let mut cleanup_interval = tokio::time::interval(Duration::from_secs(60));
        cleanup_interval.tick().await; // Skip the first immediate tick

        loop {
            tokio::select! {
                Some(msg) = message_stream.next() => {
                    // Check if we still have subscribers
                    let count = *subscriber_count.lock().await;
                    if count == 0 {
                        info!(
                            "No more subscribers for thread {} on board {}, shutting down Redis listener",
                            thread_number, board_key
                        );
                        break;
                    }

                    // Broadcast message to all WebSocket clients
                    let payload = msg.get_payload::<String>().unwrap_or_else(|_| "{}".to_string());

                    // Ignore send errors (happens when no receivers are listening)
                    let _ = broadcast_tx.send(payload);
                }
                _ = cleanup_interval.tick() => {
                    // Periodic check: verify thread is still active
                    match bbs_repo.get_thread_by_board_key_and_thread_number(&board_key, thread_number).await {
                        Ok(Some(thread)) => {
                            if !thread.active {
                                debug!(
                                    "Thread {} on board {} is no longer active, closing all connections",
                                    thread_number, board_key
                                );
                                break;
                            }
                        }
                        Ok(None) => {
                            debug!(
                                "Thread {} on board {} no longer exists, closing all connections",
                                thread_number, board_key
                            );
                            break;
                        }
                        Err(e) => {
                            error!(
                                "Failed to check thread status for thread {} on board {}: {}",
                                thread_number, board_key, e
                            );
                        }
                    }

                    // Also check if we still have subscribers
                    let count = *subscriber_count.lock().await;
                    if count == 0 {
                        info!(
                            "No more subscribers for thread {} on board {}, shutting down Redis listener",
                            thread_number, board_key
                        );
                        break;
                    }
                }
                else => {
                    info!("Redis message stream ended for thread {} on board {}", thread_number, board_key);
                    break;
                }
            }
        }

        // Cleanup: remove from subscriptions map
        {
            let mut subs = subscriptions.write().await;
            subs.remove(&channel_key);
            info!(
                "Removed subscription for thread {} on board {}",
                thread_number, board_key
            );
        }

        Ok(())
    }
}

impl<T: BbsRepository + Clone + Default> Default for WebSocketManager<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}
