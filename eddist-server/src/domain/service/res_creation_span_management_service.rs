use redis::{aio::ConnectionManager, AsyncCommands};

use crate::utils::redis::{
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

    /// Check if the timestamp is within the creation restriction span for the given authed token or ip address.
    pub async fn is_within_creation_span(
        &self,
        authed_token: &str,
        ip_adr: &str,
        timestamp: u64,
    ) -> bool {
        self.is_within_creation_span_by_authed_token(authed_token, timestamp)
            .await
            || self.is_within_creation_span_by_ip(ip_adr, timestamp).await
    }

    /// Get the current effective span for the given authed token (base span + current penalty).
    pub async fn get_effective_span_for_authed_token(&self, authed_token: &str) -> u64 {
        if self.span == 0 {
            return 0;
        }

        let mut redis_conn = self.redis_conn.clone();

        // Check if there's an active penalty
        let penalty_seconds = redis_conn
            .get::<_, u64>(&res_creation_penalty_key(authed_token))
            .await
            .unwrap_or(0);

        self.span + penalty_seconds
    }

    /// Check if the timestamp is within the creation restriction span for the given authed token.
    async fn is_within_creation_span_by_authed_token(
        &self,
        authed_token: &str,
        timestamp: u64,
    ) -> bool {
        if self.span == 0 {
            return false;
        }

        let mut redis_conn = self.redis_conn.clone();

        // Check for long-term restriction first (1 hour penalty)
        let long_restrict_exists = redis_conn
            .exists::<_, bool>(&res_creation_long_restrict_key(authed_token))
            .await
            .unwrap_or(false);

        // Check normal span restriction
        let before_res_time = redis_conn
            .get::<_, u64>(&res_creation_span_key(authed_token))
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
            .get::<_, u64>(&res_creation_penalty_key(authed_token))
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

    /// Update the last response creation time for the given authed token and ip address.
    pub async fn update_last_res_creation_time(
        &self,
        authed_token: &str,
        ip: &str,
        timestamp: u64,
    ) {
        self.update_last_res_creation_time_authed_token(authed_token, timestamp)
            .await;
        self.update_last_res_creation_time_by_ip(ip, timestamp)
            .await;
    }

    /// Update the last response creation time for the given authed token.
    async fn update_last_res_creation_time_authed_token(&self, authed_token: &str, timestamp: u64) {
        if self.span == 0 {
            return;
        }

        let mut redis_conn = self.redis_conn.clone();

        let before_res_time = redis_conn
            .get::<_, u64>(&res_creation_span_key(authed_token))
            .await
            .unwrap_or(0);

        let penalty_seconds = redis_conn
            .get::<_, u64>(&res_creation_penalty_key(authed_token))
            .await
            .unwrap_or(0);

        if before_res_time > 0 && timestamp - before_res_time < self.span * 3 {
            let new_penalty = penalty_seconds + 1;
            let max_penalty = self.span * 2; // Maximum penalty is 2x base span (so 3x total)

            log::debug!(
                "Applying penalty for authed_token {authed_token}: {penalty_seconds} -> {new_penalty} (max {max_penalty})",
            );

            if new_penalty >= max_penalty {
                // Max penalty reached - apply 1 hour restriction
                redis_conn
                    .set_ex::<_, _, ()>(
                        res_creation_long_restrict_key(authed_token),
                        timestamp,
                        60 * 60,
                    )
                    .await
                    .unwrap();

                // Clear the penalty counter since we're now in long-term restriction
                redis_conn
                    .del::<_, ()>(&res_creation_penalty_key(authed_token))
                    .await
                    .unwrap();

                log::info!(
                    "Applied long-term restriction for authed_token {authed_token} due to repeated fast creations",
                );
            } else {
                // Apply incremental penalty
                redis_conn
                    .set_ex::<_, _, ()>(
                        res_creation_penalty_key(authed_token),
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
                res_creation_span_key(authed_token),
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

    /// Check if the timestamp is within the thread creation restriction span for the given authed token or ip address.
    pub async fn is_thread_within_creation_span(
        &self,
        authed_token: &str,
        ip_adr: &str,
        timestamp: u64,
    ) -> bool {
        self.is_within_thread_creation_span_by_authed_token(authed_token, timestamp)
            .await
            || self
                .is_within_thread_creation_span_by_ip(ip_adr, timestamp)
                .await
    }

    /// Check if the timestamp is within the thread creation restriction span for the given authed token.
    async fn is_within_thread_creation_span_by_authed_token(
        &self,
        authed_token: &str,
        timestamp: u64,
    ) -> bool {
        if self.thread_span == 0 {
            return false;
        }

        let mut redis_conn = self.redis_conn.clone();

        let before_res_time = redis_conn
            .get::<_, u64>(thread_creation_span_key(authed_token))
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

    /// Update the last thread creation time for the given authed token and ip address.
    pub async fn update_last_thread_creation_time(
        &self,
        authed_token: &str,
        ip: &str,
        timestamp: u64,
    ) {
        self.update_last_thread_creation_time_authed_token(authed_token, timestamp)
            .await;
        self.update_last_thread_creation_time_by_ip(ip, timestamp)
            .await;
    }

    /// Update the last thread creation time for the given authed token.
    async fn update_last_thread_creation_time_authed_token(
        &self,
        authed_token: &str,
        timestamp: u64,
    ) {
        if self.thread_span == 0 {
            return;
        }

        let mut redis_conn = self.redis_conn.clone();

        redis_conn
            .set_ex::<_, _, ()>(
                thread_creation_span_key(authed_token),
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
