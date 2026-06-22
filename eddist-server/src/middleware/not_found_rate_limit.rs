use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use redis::AsyncCommands as _;

use eddist_core::redis_keys::not_found_access_count_key;

use crate::{AppState, utils::get_origin_ip};

/// Window over which 404 accesses are counted.
const NOT_FOUND_WINDOW_SECS: u64 = 60;
/// Max 404 accesses allowed within the window before a penalty is applied.
const NOT_FOUND_THRESHOLD: i64 = 100;
/// Duration for which a penalized IP is rejected with 429 on all endpoints.
const NOT_FOUND_PENALTY: Duration = Duration::from_secs(5 * 60);

/// In-memory cache of IPs currently penalized for excessive 404 access.
///
/// Only the 404 counter is shared via Redis (it must be consistent across
/// requests); the penalty gate itself is checked on every request to every
/// endpoint, so it is kept local to avoid a Redis round-trip per request.
#[derive(Clone, Default)]
pub struct NotFoundPenaltyCache {
    penalized_until: Arc<RwLock<HashMap<String, Instant>>>,
}

impl NotFoundPenaltyCache {
    pub fn new() -> Self {
        Self::default()
    }

    fn is_penalized(&self, ip: &str) -> bool {
        // Fast path: most requests are reads (not-penalized, or still-penalized
        // lookups), so check under a read lock first to avoid blocking other readers.
        match self.penalized_until.read().unwrap().get(ip) {
            Some(until) if *until > Instant::now() => return true,
            Some(_) => {} // expired; fall through to remove it under a write lock
            None => return false,
        }

        self.penalized_until.write().unwrap().remove(ip);
        false
    }

    fn penalize(&self, ip: &str) {
        self.penalized_until
            .write()
            .unwrap()
            .insert(ip.to_string(), Instant::now() + NOT_FOUND_PENALTY);
    }
}

/// Paths probed internally (e.g. by k8s) without going through the CDN, so they
/// never carry a `Cf-Connecting-IP`/`X-Forwarded-For` header. Skip the rate
/// limiter entirely for these rather than letting `get_origin_ip` panic.
const SKIP_PATHS: [&str; 2] = ["/health-check", "/metrics"];

pub async fn not_found_rate_limit_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    if SKIP_PATHS.contains(&request.uri().path()) {
        return next.run(request).await;
    }

    let ip = get_origin_ip(request.headers()).to_string();

    if state.not_found_penalty_cache.is_penalized(&ip) {
        tracing::warn!("IP {ip} hit 404 rate limit penalty; rejecting with 429");
        return (StatusCode::TOO_MANY_REQUESTS, "Too Many Requests").into_response();
    }

    let response = next.run(request).await;

    if response.status() == StatusCode::NOT_FOUND {
        let mut redis_conn = state.redis_conn.clone();
        let key = not_found_access_count_key(&ip);

        let count: i64 = match redis_conn.incr(&key, 1).await {
            Ok(count) => count,
            Err(e) => {
                tracing::error!("Failed to increment 404 counter for {ip}: {e}");
                return response;
            }
        };

        if count == 1
            && let Err(e) = redis_conn
                .expire::<_, ()>(&key, NOT_FOUND_WINDOW_SECS as i64)
                .await
        {
            tracing::error!("Failed to set expiry on 404 counter for {ip}: {e}");
        }

        if count > NOT_FOUND_THRESHOLD {
            tracing::warn!(
                "IP {ip} exceeded 404 rate limit ({count} hits within {NOT_FOUND_WINDOW_SECS}s); penalizing for {}s",
                NOT_FOUND_PENALTY.as_secs()
            );
            state.not_found_penalty_cache.penalize(&ip);
        }
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_penalized_by_default() {
        let cache = NotFoundPenaltyCache::new();
        assert!(!cache.is_penalized("203.0.113.1"));
    }

    #[test]
    fn test_penalize_marks_ip_as_penalized() {
        let cache = NotFoundPenaltyCache::new();
        cache.penalize("203.0.113.1");
        assert!(cache.is_penalized("203.0.113.1"));
        assert!(!cache.is_penalized("203.0.113.2"));
    }

    #[test]
    fn test_expired_penalty_is_cleared() {
        let cache = NotFoundPenaltyCache::new();
        cache.penalized_until.write().unwrap().insert(
            "203.0.113.1".to_string(),
            Instant::now() - Duration::from_secs(1),
        );

        assert!(!cache.is_penalized("203.0.113.1"));
        assert!(
            !cache
                .penalized_until
                .read()
                .unwrap()
                .contains_key("203.0.113.1")
        );
    }
}
