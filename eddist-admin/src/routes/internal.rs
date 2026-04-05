use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use eddist_core::{redis_keys::authed_token_suspended_key, utils::is_authed_token_backup_enabled};
use redis::AsyncCommands;
use serde::Deserialize;
use uuid::Uuid;

use crate::{AppState, error::ApiError};

use super::auth_tokens::{clear_require_reauth_token, require_reauth_token};

pub fn create_internal_routes() -> Router<AppState> {
    Router::new()
        .route("/authed-tokens/suspend", post(suspend_authed_token))
        .route("/authed-tokens/revoke", post(revoke_authed_token))
        .route(
            "/authed-tokens/require-reauth/{authedTokenId}",
            post(require_reauth_token).delete(clear_require_reauth_token),
        )
}

#[derive(Deserialize)]
pub struct SuspendAuthedTokenInput {
    pub authed_token_id: Uuid,
    pub ttl_seconds: u64,
}

pub async fn suspend_authed_token(
    State(state): State<AppState>,
    Json(input): Json<SuspendAuthedTokenInput>,
) -> Result<StatusCode, ApiError> {
    if input.ttl_seconds == 0 {
        return Err(ApiError::bad_request("ttl_seconds must be greater than 0"));
    }

    let authed_token = state
        .authed_token_repo
        .get_authed_token(input.authed_token_id)
        .await
        .map_err(|e| {
            if let Some(sqlx::Error::RowNotFound) = e.downcast_ref::<sqlx::Error>() {
                ApiError::not_found("authed token not found")
            } else {
                ApiError::Internal(e)
            }
        })?;

    if !authed_token.validity {
        return Err(ApiError::bad_request(
            "this token has already been permanently revoked",
        ));
    }

    let key = authed_token_suspended_key(&input.authed_token_id.to_string());
    let mut conn = state.redis_conn.clone();
    conn.set_ex::<_, _, ()>(&key, "1", input.ttl_seconds)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct RevokeAuthedTokenInput {
    pub authed_token_id: Uuid,
}

pub async fn revoke_authed_token(
    State(state): State<AppState>,
    Json(input): Json<RevokeAuthedTokenInput>,
) -> Result<StatusCode, ApiError> {
    state
        .authed_token_repo
        .delete_authed_token(input.authed_token_id)
        .await?;

    if is_authed_token_backup_enabled() {
        let mut conn = state.redis_conn.clone();
        super::auth_tokens::publish_token_revoked(&mut conn, input.authed_token_id).await;
    }

    Ok(StatusCode::NO_CONTENT)
}
