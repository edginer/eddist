use std::sync::LazyLock;

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use redis::Script;

use eddist_core::{domain::ip_addr::ReducedIpAddr, redis_keys::shared_ng_id_ip_rate_limit_key};

use crate::{AppState, utils::get_origin_ip};

const NG_ID_WINDOW_SECS: i64 = 60;

/// Sized for すべてクリア, which fires one unbatched DELETE per stored rule -
/// a legitimate burst far larger than interactive use.
const NG_ID_THRESHOLD: i64 = 300;

/// One script so `INCR` and `EXPIRE` cannot be split: a failed `EXPIRE` would
/// leave a key that never expires.
static RATE_LIMIT_SCRIPT: LazyLock<Script> = LazyLock::new(|| {
    Script::new(
        r"
        local count = redis.call('INCR', KEYS[1])
        if count == 1 then
            redis.call('EXPIRE', KEYS[1], ARGV[1])
        end
        return count
        ",
    )
});

/// Keyed on the source, not the authed token: `post_ng_id`'s per-token limit
/// needs the DB lookup it would be protecting, and `delete_ng_id` has none.
pub async fn ng_id_rate_limit_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let Some(ip) = get_origin_ip(request.headers()) else {
        return next.run(request).await;
    };
    let ip = ReducedIpAddr::from(ip.to_string()).to_string();
    let key = shared_ng_id_ip_rate_limit_key(&ip);

    let mut redis_conn = state.redis_conn.clone();
    let count: i64 = match RATE_LIMIT_SCRIPT
        .key(&key)
        .arg(NG_ID_WINDOW_SECS)
        .invoke_async(&mut redis_conn)
        .await
    {
        Ok(count) => count,
        Err(e) => {
            tracing::error!("Failed to check shared NG ID rate limit for {ip}: {e}");
            return next.run(request).await;
        }
    };

    if count > NG_ID_THRESHOLD {
        tracing::warn!(
            "IP {ip} exceeded shared NG ID rate limit ({count} requests within {NG_ID_WINDOW_SECS}s); rejecting with 429"
        );
        return (StatusCode::TOO_MANY_REQUESTS, "Too Many Requests").into_response();
    }

    next.run(request).await
}
