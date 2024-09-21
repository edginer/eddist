use redis::{aio::ConnectionManager, AsyncCommands};

#[derive(Clone)]
pub struct ResCreationSpanManagementService {
    redis_conn: ConnectionManager,
    span: u64,
}

impl ResCreationSpanManagementService {
    pub fn new(redis_conn: ConnectionManager, span: u64) -> Self {
        Self { redis_conn, span }
    }

    /// Check if the timestamp is within the creation restriction span.
    pub async fn is_within_creation_span(&self, authed_token: &str, timestamp: u64) -> bool {
        let mut redis_conn = self.redis_conn.clone();
        let span = self.span;

        let key = format!("res_creation_span:{authed_token}");
        let before_res_time = redis_conn.get::<_, u64>(&key).await.unwrap_or(0);

        timestamp - before_res_time < span
    }

    /// Update the last response creation time.
    pub async fn update_last_res_creation_time(&self, authed_token: &str, timestamp: u64) {
        let mut redis_conn = self.redis_conn.clone();

        let key = format!("res_creation_span:{authed_token}");
        redis_conn
            .set_ex::<_, _, ()>(key, timestamp, self.span)
            .await
            .unwrap();
    }
}
