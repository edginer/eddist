use eddist_core::redis_keys::{
    res_creation_long_restrict_key, res_creation_penalty_key, res_creation_span_ip_key,
    res_creation_span_key, thread_creation_span_ip_key, thread_creation_span_key,
};
use redis::{AsyncCommands, aio::ConnectionManager};

/// `auth_ip` is the authed token's reduced IP. A span of 0 disables that limit.
/// Read failures count as "not restricted"; the check and the record are separate commands.
#[async_trait::async_trait]
pub trait PostingRateLimitStore: Send + Sync {
    async fn is_res_restricted(&self, span: u64, auth_ip: &str, ip: &str, now: u64) -> bool;

    /// Includes the escalating penalty and the 1-hour restriction.
    async fn res_wait_secs(&self, span: u64, auth_ip: &str) -> u64;

    async fn record_res(&self, span: u64, auth_ip: &str, ip: &str, now: u64) -> anyhow::Result<()>;

    async fn is_thread_restricted(&self, span: u64, auth_ip: &str, ip: &str, now: u64) -> bool;

    async fn record_thread(
        &self,
        span: u64,
        auth_ip: &str,
        ip: &str,
        now: u64,
    ) -> anyhow::Result<()>;
}

#[derive(Clone)]
pub struct RedisPostingRateLimitStore(ConnectionManager);

impl RedisPostingRateLimitStore {
    pub fn new(redis_conn: ConnectionManager) -> Self {
        Self(redis_conn)
    }

    async fn is_res_restricted_by_auth_ip(&self, span: u64, auth_ip: &str, now: u64) -> bool {
        let mut conn = self.0.clone();

        let long_restrict_exists = conn
            .exists::<_, bool>(&res_creation_long_restrict_key(auth_ip))
            .await
            .unwrap_or(false);

        let before_res_time = conn
            .get::<_, u64>(&res_creation_span_key(auth_ip))
            .await
            .unwrap_or(0);

        if before_res_time == 0 {
            return false;
        }

        let time_since_last = now - before_res_time;

        if long_restrict_exists && time_since_last < span * 3 {
            return true;
        }

        let penalty_seconds = conn
            .get::<_, u64>(&res_creation_penalty_key(auth_ip))
            .await
            .unwrap_or(0);

        time_since_last < span + penalty_seconds
    }

    async fn is_res_restricted_by_ip(&self, span: u64, ip: &str, now: u64) -> bool {
        let mut conn = self.0.clone();

        let before_res_time = conn
            .get::<_, u64>(&res_creation_span_ip_key(ip))
            .await
            .unwrap_or(0);

        now - before_res_time < span
    }

    async fn record_res_by_auth_ip(
        &self,
        span: u64,
        auth_ip: &str,
        now: u64,
    ) -> anyhow::Result<()> {
        let mut conn = self.0.clone();

        let before_res_time = conn
            .get::<_, u64>(&res_creation_span_key(auth_ip))
            .await
            .unwrap_or(0);

        let penalty_seconds = conn
            .get::<_, u64>(&res_creation_penalty_key(auth_ip))
            .await
            .unwrap_or(0);

        if before_res_time > 0 && now - before_res_time < span * 3 {
            let new_penalty = penalty_seconds + 1;
            // Caps the effective span at 3x the base span.
            let max_penalty = span * 2;

            log::debug!(
                "Applying penalty for auth_ip {auth_ip}: {penalty_seconds} -> {new_penalty} (max {max_penalty})",
            );

            if new_penalty >= max_penalty {
                conn.set_ex::<_, _, ()>(res_creation_long_restrict_key(auth_ip), now, 60 * 60)
                    .await?;
                conn.del::<_, ()>(&res_creation_penalty_key(auth_ip))
                    .await?;

                log::info!(
                    "Applied long-term restriction for auth_ip {auth_ip} due to repeated fast creations",
                );
            } else {
                conn.set_ex::<_, _, ()>(
                    res_creation_penalty_key(auth_ip),
                    new_penalty,
                    span * 3 + penalty_seconds,
                )
                .await?;
            }
        }

        conn.set_ex::<_, _, ()>(
            res_creation_span_key(auth_ip),
            now,
            span * 3 + penalty_seconds,
        )
        .await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl PostingRateLimitStore for RedisPostingRateLimitStore {
    async fn is_res_restricted(&self, span: u64, auth_ip: &str, ip: &str, now: u64) -> bool {
        if span == 0 {
            return false;
        }
        self.is_res_restricted_by_auth_ip(span, auth_ip, now).await
            || self.is_res_restricted_by_ip(span, ip, now).await
    }

    async fn res_wait_secs(&self, span: u64, auth_ip: &str) -> u64 {
        if span == 0 {
            return 0;
        }

        let mut conn = self.0.clone();

        let long_restrict_exists = conn
            .exists::<_, bool>(&res_creation_long_restrict_key(auth_ip))
            .await
            .unwrap_or(false);

        if long_restrict_exists {
            return span * 3;
        }

        let penalty_seconds = conn
            .get::<_, u64>(&res_creation_penalty_key(auth_ip))
            .await
            .unwrap_or(0);

        span + penalty_seconds
    }

    async fn record_res(&self, span: u64, auth_ip: &str, ip: &str, now: u64) -> anyhow::Result<()> {
        if span == 0 {
            return Ok(());
        }
        self.record_res_by_auth_ip(span, auth_ip, now).await?;

        let mut conn = self.0.clone();
        conn.set_ex::<_, _, ()>(res_creation_span_ip_key(ip), now, span)
            .await?;
        Ok(())
    }

    async fn is_thread_restricted(&self, span: u64, auth_ip: &str, ip: &str, now: u64) -> bool {
        if span == 0 {
            return false;
        }

        let mut conn = self.0.clone();

        let before_by_auth_ip = conn
            .get::<_, u64>(thread_creation_span_key(auth_ip))
            .await
            .unwrap_or(0);
        if now - before_by_auth_ip < span {
            return true;
        }

        let before_by_ip = conn
            .get::<_, u64>(thread_creation_span_ip_key(ip))
            .await
            .unwrap_or(0);
        now - before_by_ip < span
    }

    async fn record_thread(
        &self,
        span: u64,
        auth_ip: &str,
        ip: &str,
        now: u64,
    ) -> anyhow::Result<()> {
        if span == 0 {
            return Ok(());
        }

        let mut conn = self.0.clone();
        conn.set_ex::<_, _, ()>(thread_creation_span_key(auth_ip), now, span)
            .await?;
        conn.set_ex::<_, _, ()>(thread_creation_span_ip_key(ip), now, span)
            .await?;
        Ok(())
    }
}
