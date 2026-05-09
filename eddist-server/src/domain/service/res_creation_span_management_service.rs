use redis::{AsyncCommands, aio::ConnectionManager};

use eddist_core::redis_keys::{
    res_creation_long_restrict_key, res_creation_penalty_key, res_creation_span_ip_key,
    res_creation_span_key, thread_creation_span_ip_key, thread_creation_span_key,
};

#[derive(Clone)]
pub struct ResCreationSpanManagementService {
    redis_conn: ConnectionManager,
    span: u64,
    thread_span: u64,
}

impl ResCreationSpanManagementService {
    pub fn new(redis_conn: ConnectionManager, span: u64, thread_span: u64) -> Self {
        Self {
            redis_conn,
            span,
            thread_span,
        }
    }

    /// Check if the timestamp is within the creation restriction span for the given auth IP or posting IP.
    pub async fn is_within_creation_span(
        &self,
        auth_ip: &str,
        ip_adr: &str,
        timestamp: u64,
    ) -> bool {
        self.is_within_creation_span_by_auth_ip(auth_ip, timestamp)
            .await
            || self.is_within_creation_span_by_ip(ip_adr, timestamp).await
    }

    /// Get the actual wait time for the given auth IP considering all restrictions.
    /// Returns the number of seconds the user needs to wait before their next post.
    pub async fn get_actual_wait_time_for_auth_ip(&self, auth_ip: &str) -> u64 {
        if self.span == 0 {
            return 0;
        }

        let mut redis_conn = self.redis_conn.clone();

        let long_restrict_exists = redis_conn
            .exists::<_, bool>(&res_creation_long_restrict_key(auth_ip))
            .await
            .unwrap_or(false);

        if long_restrict_exists {
            return self.span * 3;
        }

        let penalty_seconds = redis_conn
            .get::<_, u64>(&res_creation_penalty_key(auth_ip))
            .await
            .unwrap_or(0);

        self.span + penalty_seconds
    }

    /// Check if the timestamp is within the creation restriction span for the given auth IP.
    async fn is_within_creation_span_by_auth_ip(&self, auth_ip: &str, timestamp: u64) -> bool {
        if self.span == 0 {
            return false;
        }

        let mut redis_conn = self.redis_conn.clone();

        // Check for long-term restriction first (1 hour penalty)
        let long_restrict_exists = redis_conn
            .exists::<_, bool>(&res_creation_long_restrict_key(auth_ip))
            .await
            .unwrap_or(false);

        // Check normal span restriction
        let before_res_time = redis_conn
            .get::<_, u64>(&res_creation_span_key(auth_ip))
            .await
            .unwrap_or(0);

        if before_res_time == 0 {
            return false;
        }

        let time_since_last = timestamp - before_res_time;

        if long_restrict_exists && time_since_last < self.span * 3 {
            return true;
        }

        // Check if there's an active penalty
        let penalty_seconds = redis_conn
            .get::<_, u64>(&res_creation_penalty_key(auth_ip))
            .await
            .unwrap_or(0);

        // Calculate effective span (base span + penalty)
        let effective_span = self.span + penalty_seconds;

        time_since_last < effective_span
    }

    /// Check if the timestamp is within the creation restriction span for the given ip address.
    async fn is_within_creation_span_by_ip(&self, ip: &str, timestamp: u64) -> bool {
        if self.span == 0 {
            return false;
        }

        let mut redis_conn = self.redis_conn.clone();
        let span = self.span;

        let before_res_time = redis_conn
            .get::<_, u64>(&res_creation_span_ip_key(ip))
            .await
            .unwrap_or(0);

        timestamp - before_res_time < span
    }

    /// Update the last response creation time for the given auth IP and posting IP.
    pub async fn update_last_res_creation_time(&self, auth_ip: &str, ip: &str, timestamp: u64) {
        self.update_last_res_creation_time_by_auth_ip(auth_ip, timestamp)
            .await;
        self.update_last_res_creation_time_by_ip(ip, timestamp)
            .await;
    }

    /// Update the last response creation time keyed by the token's authentication IP.
    async fn update_last_res_creation_time_by_auth_ip(&self, auth_ip: &str, timestamp: u64) {
        if self.span == 0 {
            return;
        }

        let mut redis_conn = self.redis_conn.clone();

        let before_res_time = redis_conn
            .get::<_, u64>(&res_creation_span_key(auth_ip))
            .await
            .unwrap_or(0);

        let penalty_seconds = redis_conn
            .get::<_, u64>(&res_creation_penalty_key(auth_ip))
            .await
            .unwrap_or(0);

        if before_res_time > 0 && timestamp - before_res_time < self.span * 3 {
            let new_penalty = penalty_seconds + 1;
            let max_penalty = self.span * 2; // Maximum penalty is 2x base span (so 3x total)

            log::debug!(
                "Applying penalty for auth_ip {auth_ip}: {penalty_seconds} -> {new_penalty} (max {max_penalty})",
            );

            if new_penalty >= max_penalty {
                // Max penalty reached - apply 1 hour restriction
                redis_conn
                    .set_ex::<_, _, ()>(res_creation_long_restrict_key(auth_ip), timestamp, 60 * 60)
                    .await
                    .unwrap();

                // Clear the penalty counter since we're now in long-term restriction
                redis_conn
                    .del::<_, ()>(&res_creation_penalty_key(auth_ip))
                    .await
                    .unwrap();

                log::info!(
                    "Applied long-term restriction for auth_ip {auth_ip} due to repeated fast creations",
                );
            } else {
                // Apply incremental penalty
                redis_conn
                    .set_ex::<_, _, ()>(
                        res_creation_penalty_key(auth_ip),
                        new_penalty,
                        self.span * 3 + penalty_seconds,
                    )
                    .await
                    .unwrap();
            }
        }

        // Update the last creation time
        redis_conn
            .set_ex::<_, _, ()>(
                res_creation_span_key(auth_ip),
                timestamp,
                self.span * 3 + penalty_seconds,
            )
            .await
            .unwrap();
    }

    /// Update the last response creation time for the given ip address.
    async fn update_last_res_creation_time_by_ip(&self, ip: &str, timestamp: u64) {
        if self.span == 0 {
            return;
        }

        let mut redis_conn = self.redis_conn.clone();

        redis_conn
            .set_ex::<_, _, ()>(res_creation_span_ip_key(ip), timestamp, self.span)
            .await
            .unwrap();
    }

    /// Check if the timestamp is within the thread creation restriction span for the given auth IP or posting IP.
    pub async fn is_thread_within_creation_span(
        &self,
        auth_ip: &str,
        ip_adr: &str,
        timestamp: u64,
    ) -> bool {
        self.is_within_thread_creation_span_by_auth_ip(auth_ip, timestamp)
            .await
            || self
                .is_within_thread_creation_span_by_ip(ip_adr, timestamp)
                .await
    }

    /// Check if the timestamp is within the thread creation restriction span for the given auth IP.
    async fn is_within_thread_creation_span_by_auth_ip(
        &self,
        auth_ip: &str,
        timestamp: u64,
    ) -> bool {
        if self.thread_span == 0 {
            return false;
        }

        let mut redis_conn = self.redis_conn.clone();

        let before_res_time = redis_conn
            .get::<_, u64>(thread_creation_span_key(auth_ip))
            .await
            .unwrap_or(0);

        timestamp - before_res_time < self.thread_span
    }

    /// Check if the timestamp is within the thread creation restriction span for the given ip address.
    async fn is_within_thread_creation_span_by_ip(&self, ip: &str, timestamp: u64) -> bool {
        if self.thread_span == 0 {
            return false;
        }

        let mut redis_conn = self.redis_conn.clone();

        let before_res_time = redis_conn
            .get::<_, u64>(thread_creation_span_ip_key(ip))
            .await
            .unwrap_or(0);

        timestamp - before_res_time < self.thread_span
    }

    /// Update the last thread creation time for the given auth IP and posting IP.
    pub async fn update_last_thread_creation_time(&self, auth_ip: &str, ip: &str, timestamp: u64) {
        self.update_last_thread_creation_time_by_auth_ip(auth_ip, timestamp)
            .await;
        self.update_last_thread_creation_time_by_ip(ip, timestamp)
            .await;
    }

    /// Update the last thread creation time keyed by the token's authentication IP.
    async fn update_last_thread_creation_time_by_auth_ip(&self, auth_ip: &str, timestamp: u64) {
        if self.thread_span == 0 {
            return;
        }

        let mut redis_conn = self.redis_conn.clone();

        redis_conn
            .set_ex::<_, _, ()>(
                thread_creation_span_key(auth_ip),
                timestamp,
                self.thread_span,
            )
            .await
            .unwrap();
    }

    /// Update the last thread creation time for the given ip address.
    async fn update_last_thread_creation_time_by_ip(&self, ip: &str, timestamp: u64) {
        if self.thread_span == 0 {
            return;
        }

        let mut redis_conn = self.redis_conn.clone();

        redis_conn
            .set_ex::<_, _, ()>(thread_creation_span_ip_key(ip), timestamp, self.thread_span)
            .await
            .unwrap();
    }
}
