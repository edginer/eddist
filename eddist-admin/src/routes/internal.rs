use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
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

    state
        .services
        .authed_token
        .suspend_authed_token(input.authed_token_id, input.ttl_seconds)
        .await?;

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
    // Internal routes don't have a session-based actor; use a system placeholder.
    let system_actor = crate::auth::AdminIdentity {
        sub: "system".to_string(),
        email: "system@internal".to_string(),
        username: "system".to_string(),
    };
    state
        .services
        .authed_token
        .revoke_authed_token(&system_actor, input.authed_token_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
