use axum::{
    Json,
    body::Body,
    extract::{Path, State},
    response::Response,
};
use axum_extra::extract::CookieJar;
use eddist_core::{domain::board::validate_board_key, utils::to_hex};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    AppState,
    services::{AppService, edge_token_validation_service::EdgeTokenValidationServiceInput},
    stores::shared_ng_store::SharedNgKind,
};

const MAX_SHARED_VALUE_LEN: usize = 64;

const THREAD_METADENT_LEN: usize = 8;

const MAX_NG_IDS_PER_DELETE: usize = 200;

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

async fn add_shared_value(
    state: &AppState,
    jar: &CookieJar,
    kind: SharedNgKind,
    board_key: &str,
    value: &str,
    kind_label: &str,
) -> Response {
    let hash = match resolve_contributor_hash(state, jar).await {
        Ok(hash) => hash,
        Err(resp) => return resp,
    };

    let store = &state.shared_ng;
    match store.take_contribution_quota(&hash).await {
        Ok(true) => {}
        Ok(false) => return empty(429),
        Err(e) => {
            log::error!("Failed to check shared {kind_label} rate limit: {e:?}");
            return empty(500);
        }
    }

    let contributors = match store.add_contributor(kind, board_key, value, &hash).await {
        Ok(contributors) => contributors,
        Err(e) => {
            log::error!("Failed to add shared {kind_label} to Redis: {e:?}");
            return empty(500);
        }
    };

    if contributors >= SHARED_NG_PROMOTION_THRESHOLD
        && let Err(e) = store
            .promote(kind, board_key, value, chrono::Utc::now().timestamp())
            .await
    {
        // The contribution itself landed, so report success and let the next
        // contribution re-attempt the promotion.
        log::error!("Failed to promote shared {kind_label} to the board-wide list: {e:?}");
    }

    empty(204)
}

async fn remove_shared_values(
    state: &AppState,
    jar: &CookieJar,
    kind: SharedNgKind,
    board_key: &str,
    values: &[String],
    kind_label: &str,
) -> Response {
    let hash = match resolve_contributor_hash(state, jar).await {
        Ok(hash) => hash,
        Err(resp) => return resp,
    };

    if let Err(e) = state
        .shared_ng
        .remove_contributor(kind, board_key, values, &hash)
        .await
    {
        log::error!("Failed to remove shared {kind_label} from Redis: {e:?}");
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

    let active = match state
        .shared_ng
        .active(
            &board_key,
            chrono::Utc::now().timestamp(),
            MAX_SHARED_NG_RESPONSE_LEN,
        )
        .await
    {
        Ok(active) => active,
        Err(e) => {
            log::error!("Failed to read shared NG lists from Redis: {e:?}");
            return empty(500);
        }
    };

    let body = SharedNgResponse {
        ng_ids: active.author_ids,
        thread_metadents: active.thread_metadents,
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

    add_shared_value(
        &state,
        &jar,
        SharedNgKind::AuthorId,
        &board_key,
        &req.ng_id,
        "NG ID",
    )
    .await
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

    remove_shared_values(
        &state,
        &jar,
        SharedNgKind::AuthorId,
        &board_key,
        &req.ng_ids,
        "NG IDs",
    )
    .await
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

    add_shared_value(
        &state,
        &jar,
        SharedNgKind::ThreadMetadent,
        &board_key,
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

    remove_shared_values(
        &state,
        &jar,
        SharedNgKind::ThreadMetadent,
        &board_key,
        &req.metadents,
        "thread metadents",
    )
    .await
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
