use axum::{
    Json,
    body::Body,
    extract::{Path, State},
    response::Response,
};
use axum_extra::extract::CookieJar;
use eddist_core::{
    domain::board::validate_board_key,
    redis_keys::{
        shared_ng_id_active_key, shared_ng_id_key, shared_ng_id_rate_limit_key,
        shared_ng_thread_metadent_active_key, shared_ng_thread_metadent_key,
    },
    utils::to_hex,
};
use redis::pipe;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    AppState,
    services::{AppService, edge_token_validation_service::EdgeTokenValidationServiceInput},
    utils::incr_fixed_window,
};

// Shared NG marks are intentionally short-lived. They represent recent community
// signals rather than permanent moderation decisions.
const SHARED_NG_TTL_SECS: i64 = 3 * 24 * 60 * 60;

const MAX_SHARED_VALUE_LEN: usize = 64;

const THREAD_METADENT_LEN: usize = 8;

const MAX_NG_IDS_PER_DELETE: usize = 200;

const SHARED_NG_RATE_LIMIT: i64 = 25;
const SHARED_NG_RATE_LIMIT_WINDOW_SECS: i64 = 24 * 60 * 60;

// How many distinct contributors a value needs before it is served to every reader
// of the board.
const SHARED_NG_PROMOTION_THRESHOLD: usize = 5;

const MAX_SHARED_NG_RESPONSE_LEN: isize = 500;

#[derive(Deserialize)]
pub struct AddNgIdRequest {
    pub ng_id: String,
}

#[derive(Deserialize)]
pub struct AddThreadMetadentRequest {
    pub metadent: String,
}

#[derive(Deserialize)]
pub struct DeleteNgIdsRequest {
    pub ng_ids: Vec<String>,
}

#[derive(Deserialize)]
pub struct DeleteThreadMetadentsRequest {
    pub metadents: Vec<String>,
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
    to_hex(hasher.finalize())
}

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
            log::error!("Failed to validate edge-token for shared NG mark: {e:?}");
            Err(empty(500))
        }
    }
}

fn is_valid_shared_value(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_SHARED_VALUE_LEN
        && !value.contains(':')
        && !value.chars().any(|c| c.is_control())
}

fn is_valid_ng_id(ng_id: &str) -> bool {
    is_valid_shared_value(ng_id)
}

fn is_valid_thread_metadent(metadent: &str) -> bool {
    metadent.len() == THREAD_METADENT_LEN
        && metadent
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '/'))
}

async fn within_shared_ng_rate_limit(
    conn: &mut redis::aio::ConnectionManager,
    token_hash: &str,
) -> redis::RedisResult<bool> {
    let rate_key = shared_ng_id_rate_limit_key(token_hash);
    let count = incr_fixed_window(conn, &rate_key, SHARED_NG_RATE_LIMIT_WINDOW_SECS).await?;

    Ok(count <= SHARED_NG_RATE_LIMIT)
}

async fn add_shared_value(
    state: &AppState,
    jar: &CookieJar,
    key: String,
    active_key: String,
    value: &str,
    kind: &str,
) -> Response {
    let hash = match resolve_contributor_hash(state, jar).await {
        Ok(hash) => hash,
        Err(resp) => return resp,
    };

    let mut conn = state.redis_conn.clone();

    // The rate-limit key is shared by author IDs and thread metadents, so adding
    // the second signal type cannot be used to bypass the mutation limit.
    match within_shared_ng_rate_limit(&mut conn, &hash).await {
        Ok(true) => {}
        Ok(false) => return empty(429),
        Err(e) => {
            log::error!("Failed to check shared {kind} rate limit: {e:?}");
            return empty(500);
        }
    }

    let contributors = match pipe()
        .sadd(&key, &hash)
        .ignore()
        .expire(&key, SHARED_NG_TTL_SECS)
        .ignore()
        .scard(&key)
        .query_async::<(usize,)>(&mut conn)
        .await
    {
        Ok((contributors,)) => contributors,
        Err(e) => {
            log::error!("Failed to add shared {kind} to Redis: {e:?}");
            return empty(500);
        }
    };

    if contributors >= SHARED_NG_PROMOTION_THRESHOLD
        && let Err(e) = pipe()
            .zadd(&active_key, value, chrono::Utc::now().timestamp())
            .ignore()
            .expire(&active_key, SHARED_NG_TTL_SECS)
            .ignore()
            .query_async::<()>(&mut conn)
            .await
    {
        // The contribution itself landed, so report success and let the next
        // contribution re-attempt the promotion.
        log::error!("Failed to promote shared {kind} to the board-wide list: {e:?}");
    }

    empty(204)
}

async fn remove_shared_values(
    state: &AppState,
    jar: &CookieJar,
    keys: Vec<String>,
    kind: &str,
) -> Response {
    let hash = match resolve_contributor_hash(state, jar).await {
        Ok(hash) => hash,
        Err(resp) => return resp,
    };

    let mut conn = state.redis_conn.clone();
    let mut pipe = pipe();
    for key in &keys {
        // SREM is a no-op if the member/key is missing.
        pipe.srem(key, &hash).ignore();
    }

    if let Err(e) = pipe.query_async::<()>(&mut conn).await {
        log::error!("Failed to remove shared {kind} from Redis: {e:?}");
        return empty(500);
    }

    empty(204)
}

#[derive(Serialize)]
pub struct SharedNgResponse {
    pub ng_ids: Vec<String>,
    pub thread_metadents: Vec<String>,
}

pub async fn get_shared_ng(
    State(state): State<AppState>,
    Path(board_key): Path<String>,
) -> Response {
    if validate_board_key(&board_key).is_err() {
        return empty(404);
    }

    let ng_id_key = shared_ng_id_active_key(&board_key);
    let metadent_key = shared_ng_thread_metadent_active_key(&board_key);
    // Promotions are only refreshed on contribution, so entries older than the
    // contributor sets' own TTL are dropped here rather than being served forever.
    let cutoff = chrono::Utc::now().timestamp() - SHARED_NG_TTL_SECS;
    let last = MAX_SHARED_NG_RESPONSE_LEN - 1;

    let mut conn = state.redis_conn.clone();
    let (ng_ids, thread_metadents) = match pipe()
        .zrembyscore(&ng_id_key, "-inf", cutoff)
        .ignore()
        .zrembyscore(&metadent_key, "-inf", cutoff)
        .ignore()
        .zrevrange(&ng_id_key, 0, last)
        .zrevrange(&metadent_key, 0, last)
        .query_async::<(Vec<String>, Vec<String>)>(&mut conn)
        .await
    {
        Ok(lists) => lists,
        Err(e) => {
            log::error!("Failed to read shared NG lists from Redis: {e:?}");
            return empty(500);
        }
    };

    let body = SharedNgResponse {
        ng_ids,
        thread_metadents,
    };

    match serde_json::to_vec(&body) {
        Ok(json) => Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .header("Cache-Control", "public, max-age=60")
            .body(Body::from(json))
            .unwrap(),
        Err(e) => {
            log::error!("Failed to serialize shared NG lists: {e:?}");
            empty(500)
        }
    }
}

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

    let key = shared_ng_id_key(&board_key, &req.ng_id);
    let active_key = shared_ng_id_active_key(&board_key);
    add_shared_value(&state, &jar, key, active_key, &req.ng_id, "NG ID").await
}

pub async fn delete_ng_ids(
    State(state): State<AppState>,
    Path(board_key): Path<String>,
    jar: CookieJar,
    Json(req): Json<DeleteNgIdsRequest>,
) -> Response {
    if validate_board_key(&board_key).is_err() {
        return empty(404);
    }
    if req.ng_ids.is_empty() || req.ng_ids.len() > MAX_NG_IDS_PER_DELETE {
        return empty(400);
    }
    if !req.ng_ids.iter().all(|ng_id| is_valid_ng_id(ng_id)) {
        return empty(400);
    }

    let keys = req
        .ng_ids
        .iter()
        .map(|ng_id| shared_ng_id_key(&board_key, ng_id))
        .collect();
    remove_shared_values(&state, &jar, keys, "NG IDs").await
}

pub async fn post_thread_metadent(
    State(state): State<AppState>,
    Path(board_key): Path<String>,
    jar: CookieJar,
    Json(req): Json<AddThreadMetadentRequest>,
) -> Response {
    if validate_board_key(&board_key).is_err() {
        return empty(404);
    }
    if !is_valid_thread_metadent(&req.metadent) {
        return empty(400);
    }

    let key = shared_ng_thread_metadent_key(&board_key, &req.metadent);
    let active_key = shared_ng_thread_metadent_active_key(&board_key);
    add_shared_value(
        &state,
        &jar,
        key,
        active_key,
        &req.metadent,
        "thread metadent",
    )
    .await
}

pub async fn delete_thread_metadents(
    State(state): State<AppState>,
    Path(board_key): Path<String>,
    jar: CookieJar,
    Json(req): Json<DeleteThreadMetadentsRequest>,
) -> Response {
    if validate_board_key(&board_key).is_err() {
        return empty(404);
    }
    if req.metadents.is_empty() || req.metadents.len() > MAX_NG_IDS_PER_DELETE {
        return empty(400);
    }
    if !req
        .metadents
        .iter()
        .all(|metadent| is_valid_thread_metadent(metadent))
    {
        return empty(400);
    }

    let keys = req
        .metadents
        .iter()
        .map(|metadent| shared_ng_thread_metadent_key(&board_key, metadent))
        .collect();
    remove_shared_values(&state, &jar, keys, "thread metadents").await
}

#[cfg(test)]
mod probe_tests {
    use super::*;

    #[test]
    fn known_vector_stable_across_digest_crate_bump() {
        assert_eq!(
            hash_edge_token("some-edge-token"),
            "2160da207a387b8210efd4de4d0c25645d9cd17f9747fa4e69f23a8bff8874b4"
        );
    }

    #[test]
    fn thread_metadent_validation_accepts_generated_format() {
        assert!(is_valid_thread_metadent("abcd1234"));
        assert!(is_valid_thread_metadent("+/ab1234"));
        assert!(is_valid_thread_metadent("abc.1234"));
    }

    #[test]
    fn thread_metadent_validation_rejects_other_values() {
        assert!(!is_valid_thread_metadent("abcd123"));
        assert!(!is_valid_thread_metadent("abcd:234"));
        assert!(!is_valid_thread_metadent("abcd 234"));
    }
}
