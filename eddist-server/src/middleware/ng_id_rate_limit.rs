use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};

use eddist_core::{domain::ip_addr::ReducedIpAddr, redis_keys::shared_ng_id_ip_rate_limit_key};

use crate::{
    AppState,
    utils::{get_origin_ip, incr_fixed_window},
};

const SHARED_NG_WINDOW_SECS: i64 = 60;

const SHARED_NG_THRESHOLD: i64 = 60;

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
    let count = match incr_fixed_window(&mut redis_conn, &key, SHARED_NG_WINDOW_SECS).await {
        Ok(count) => count,
        Err(e) => {
            tracing::error!("Failed to check shared NG mutation rate limit for {ip}: {e}");
            return next.run(request).await;
        }
    };

    if count > SHARED_NG_THRESHOLD {
        tracing::warn!(
            "IP {ip} exceeded shared NG mutation rate limit ({count} requests within {SHARED_NG_WINDOW_SECS}s); rejecting with 429"
        );
        return (StatusCode::TOO_MANY_REQUESTS, "Too Many Requests").into_response();
    }

    next.run(request).await
}
