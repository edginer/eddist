use redis::{aio::ConnectionManager, AsyncCommands};

use crate::utils::redis::{
    res_creation_span_ip_key, res_creation_span_key, thread_creation_span_ip_key,
    thread_creation_span_key,
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
        let span = self.span;

        let before_res_time = redis_conn
            .get::<_, u64>(&res_creation_span_key(authed_token))
            .await
            .unwrap_or(0);

        timestamp - before_res_time < span
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

        redis_conn
            .set_ex::<_, _, ()>(res_creation_span_key(authed_token), timestamp, self.span)
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
