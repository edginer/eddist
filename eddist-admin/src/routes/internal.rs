use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use redis::AsyncCommands;
use serde::Deserialize;
use uuid::Uuid;

use crate::{error::ApiError, AppState};

pub fn create_internal_routes() -> Router<AppState> {
    Router::new().route("/authed-tokens/suspend", post(suspend_authed_token))
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

    let key = format!("authed_token:suspended:{}", input.authed_token_id);
    let mut conn = state.redis_conn.clone();
    conn.set_ex::<_, _, ()>(&key, "1", input.ttl_seconds)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;

    Ok(StatusCode::NO_CONTENT)
}
