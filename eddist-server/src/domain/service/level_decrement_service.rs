use crate::utils::redis::level_decrement_rate_limit_key;
use redis::{aio::ConnectionManager, AsyncCommands};

#[derive(Clone)]
pub struct LevelDecrementService {
    redis_conn: ConnectionManager,
}

impl LevelDecrementService {
    pub fn new(redis_conn: ConnectionManager) -> Self {
        Self { redis_conn }
    }

    /// Check if user has exceeded 5 responses in 30 seconds and should have level decremented
    /// Returns true if level should be decremented
    pub async fn check_and_increment_response_count(
        &self,
        authed_token: &str,
        timestamp: u64,
    ) -> bool {
        let key = level_decrement_rate_limit_key(authed_token);
        let mut redis_conn = self.redis_conn.clone();

        // Get current count for this 30-second window
        let window_start = (timestamp / 30) * 30; // Round down to 30-second window
        let window_key = format!("{}:{}", key, window_start);

        // Get current count and increment
        let current_count: u32 = redis_conn.get(&window_key).await.unwrap_or(0);

        let new_count = current_count + 1;

        // Set with 30-second expiration from window start
        let expiration = 30 - (timestamp - window_start);
        redis_conn
            .set_ex::<_, _, ()>(&window_key, new_count, expiration)
            .await
            .unwrap();

        // Return true if exceeded threshold (more than 5)
        new_count > 5
    }
}
