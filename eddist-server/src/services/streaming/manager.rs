use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

/// Capacity of each broadcast channel (number of messages to buffer before lagging)
const CHANNEL_CAPACITY: usize = 100;

/// Manages broadcast channels for real-time thread updates
#[derive(Clone)]
pub struct StreamManager {
    /// Map of thread_id -> broadcast sender
    channels: Arc<DashMap<Uuid, broadcast::Sender<String>>>,
}

impl StreamManager {
    pub fn new() -> Self {
        Self {
            channels: Arc::new(DashMap::new()),
        }
    }

    /// Get a receiver for a specific thread.
    /// Creates the channel if it doesn't exist.
    pub fn subscribe(&self, thread_id: Uuid) -> broadcast::Receiver<String> {
        let sender = self
            .channels
            .entry(thread_id)
            .or_insert_with(|| {
                let (tx, _) = broadcast::channel(CHANNEL_CAPACITY);
                tx
            })
            .value()
            .clone();

        sender.subscribe()
    }

    /// Publish a message to a specific thread.
    /// Returns number of subscribers that received it.
    pub fn publish(&self, thread_id: Uuid, message: String) -> usize {
        if let Some(sender) = self.channels.get(&thread_id) {
            // send() returns error if no active receivers, which is expected
            return sender.send(message).unwrap_or(0);
        }
        0
    }

    /// Clean up empty channels to prevent memory leaks.
    /// Call this periodically or on client disconnect.
    pub fn cleanup_unused(&self) {
        self.channels
            .retain(|_, sender| sender.receiver_count() > 0);
    }
}

impl Default for StreamManager {
    fn default() -> Self {
        Self::new()
    }
}
