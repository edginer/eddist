use axum::{
    Json,
    body::Body,
    extract::{Path, State},
    response::Response,
};
use axum_extra::extract::CookieJar;
use eddist_core::{
    domain::board::validate_board_key,
    redis_keys::{shared_ng_id_key, shared_ng_id_rate_limit_key},
};
use redis::AsyncCommands as _;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::{
    AppState,
    services::{AppService, edge_token_validation_service::EdgeTokenValidationServiceInput},
};

// TTL for a shared NG ID entry, refreshed on every add.
const SHARED_NG_ID_TTL_SECS: i64 = 3 * 24 * 60 * 60;

const MAX_NG_ID_LEN: usize = 64;

// Max shared NG ID adds per authed token per rolling day.
const SHARED_NG_ID_RATE_LIMIT: i64 = 25;
const SHARED_NG_ID_RATE_LIMIT_WINDOW_SECS: i64 = 24 * 60 * 60;

#[derive(Deserialize)]
pub struct AddNgIdRequest {
    pub ng_id: String,
}

fn empty(status: u16) -> Response {
    Response::builder()
        .status(status)
        .body(Body::empty())
        .unwrap()
}

fn hash_edge_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Resolves the caller's `edge-token` cookie to the hash stored in Redis.
///
/// The cookie is attacker-controlled, so it is validated against `authed_tokens`
/// first: without that, any client could mint unlimited distinct hashes, which are
/// both the unit the contributor sets count and the rate limit's key.
async fn resolve_contributor_hash(state: &AppState, jar: &CookieJar) -> Result<String, Response> {
    let Some(edge_token) = jar.get("edge-token").map(|c| c.value().to_string()) else {
        return Err(empty(401));
    };

    let authed_token = state
        .get_container()
        .edge_token_validation()
        .execute(EdgeTokenValidationServiceInput { edge_token })
        .await;

    match authed_token {
        Ok(Some(token)) => Ok(hash_edge_token(&token.token)),
        Ok(None) => Err(empty(401)),
        Err(e) => {
            log::error!("Failed to validate edge-token for shared NG ID: {e:?}");
            Err(empty(500))
        }
    }
}

fn is_valid_ng_id(ng_id: &str) -> bool {
    !ng_id.is_empty()
        && ng_id.len() <= MAX_NG_ID_LEN
        && !ng_id.contains(':')
        && !ng_id.chars().any(|c| c.is_control())
}

async fn within_shared_ng_id_rate_limit(
    conn: &mut redis::aio::ConnectionManager,
    token_hash: &str,
) -> redis::RedisResult<bool> {
    let rate_key = shared_ng_id_rate_limit_key(token_hash);
    let count: i64 = conn.incr(&rate_key, 1).await?;
    if count == 1 {
        conn.expire::<_, ()>(&rate_key, SHARED_NG_ID_RATE_LIMIT_WINDOW_SECS)
            .await?;
    }
    Ok(count <= SHARED_NG_ID_RATE_LIMIT)
}

// POST /api/{boardKey}/ng-ids — record that the caller marked `ng_id` as NG.
pub async fn post_ng_id(
    State(state): State<AppState>,
    Path(board_key): Path<String>,
    jar: CookieJar,
    Json(req): Json<AddNgIdRequest>,
) -> Response {
    if validate_board_key(&board_key).is_err() {
        return empty(404);
    }
    if !is_valid_ng_id(&req.ng_id) {
        return empty(400);
    }

    let hash = match resolve_contributor_hash(&state, &jar).await {
        Ok(hash) => hash,
        Err(resp) => return resp,
    };

    let key = shared_ng_id_key(&board_key, &req.ng_id);
    let mut conn = state.redis_conn.clone();

    match within_shared_ng_id_rate_limit(&mut conn, &hash).await {
        Ok(true) => {}
        Ok(false) => return empty(429),
        Err(e) => {
            log::error!("Failed to check shared NG ID rate limit: {e:?}");
            return empty(500);
        }
    }

    if let Err(e) = conn.sadd::<_, _, ()>(&key, &hash).await {
        log::error!("Failed to add shared NG ID to Redis: {e:?}");
        return empty(500);
    }
    if let Err(e) = conn.expire::<_, ()>(&key, SHARED_NG_ID_TTL_SECS).await {
        log::error!("Failed to set TTL on shared NG ID: {e:?}");
        return empty(500);
    }

    empty(204)
}

// DELETE /api/{boardKey}/ng-ids/{ngId} — retract the caller's shared NG ID mark.
pub async fn delete_ng_id(
    State(state): State<AppState>,
    Path((board_key, ng_id)): Path<(String, String)>,
    jar: CookieJar,
) -> Response {
    if validate_board_key(&board_key).is_err() {
        return empty(404);
    }
    if !is_valid_ng_id(&ng_id) {
        return empty(400);
    }

    let hash = match resolve_contributor_hash(&state, &jar).await {
        Ok(hash) => hash,
        Err(resp) => return resp,
    };

    let key = shared_ng_id_key(&board_key, &ng_id);
    let mut conn = state.redis_conn.clone();

    // SREM is a no-op if the member/key is missing.
    if let Err(e) = conn.srem::<_, _, ()>(&key, &hash).await {
        log::error!("Failed to remove shared NG ID from Redis: {e:?}");
        return empty(500);
    }

    empty(204)
}
