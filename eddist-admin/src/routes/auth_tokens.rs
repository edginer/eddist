use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
};
use uuid::Uuid;

use crate::{
    AppState,
    auth::AdminIdentity,
    error::ApiError,
    models::{DeleteAuthedTokenInput, ListAuthedTokensQuery, PaginatedAuthedTokens},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/authed_tokens", get(list_authed_tokens))
        .route("/authed_tokens/{authedTokenId}", get(get_authed_token))
        .route(
            "/authed_tokens/{authedTokenId}",
            delete(delete_authed_token),
        )
        .route(
            "/authed_tokens/{authedTokenId}/require-reauth",
            post(require_reauth_token).delete(clear_require_reauth_token),
        )
}

#[utoipa::path(
    get,
    path = "/authed_tokens",
    responses(
        (status = 200, description = "List authed tokens successfully", body = PaginatedAuthedTokens),
    ),
    params(
        ListAuthedTokensQuery,
    ),
)]
pub async fn list_authed_tokens(
    State(state): State<AppState>,
    Query(query): Query<ListAuthedTokensQuery>,
) -> Result<Json<PaginatedAuthedTokens>, ApiError> {
    let result = state
        .services
        .authed_token
        .list_authed_tokens(query)
        .await?;
    Ok(Json(result))
}

#[utoipa::path(
    get,
    path = "/authed_tokens/{authed_token_id}/",
    responses(
        (status = 200, description = "Get authed token successfully", body = crate::models::AuthedToken),
    ),
    params(
        ("authed_token_id" = Uuid, Path, description = "Authed token ID"),
    ),
)]
pub async fn get_authed_token(
    State(state): State<AppState>,
    Path(authed_token_id): Path<Uuid>,
) -> Result<Json<crate::models::AuthedToken>, ApiError> {
    let authed_token = state
        .services
        .authed_token
        .get_authed_token(authed_token_id)
        .await?;
    Ok(Json(authed_token))
}

#[utoipa::path(
    delete,
    path = "/authed_tokens/{authed_token_id}/",
    responses(
        (status = 200, description = "Delete authed token successfully"),
    ),
    params(
        ("authed_token_id" = Uuid, Path, description = "Authed token ID"),
        DeleteAuthedTokenInput
    ),
)]
pub async fn delete_authed_token(
    State(state): State<AppState>,
    identity: AdminIdentity,
    Path(authed_token_id): Path<Uuid>,
    Query(options): Query<DeleteAuthedTokenInput>,
) -> Result<StatusCode, ApiError> {
    state
        .services
        .authed_token
        .delete_authed_token(&identity, authed_token_id, options)
        .await?;
    Ok(StatusCode::OK)
}

#[utoipa::path(
    post,
    path = "/authed_tokens/{authed_token_id}/require-reauth",
    responses(
        (status = 204, description = "Set require re-auth successfully"),
    ),
    params(
        ("authed_token_id" = Uuid, Path, description = "Authed token ID"),
    ),
)]
pub async fn require_reauth_token(
    State(state): State<AppState>,
    Path(authed_token_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    state
        .services
        .authed_token
        .set_require_reauth(authed_token_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    delete,
    path = "/authed_tokens/{authed_token_id}/require-reauth",
    responses(
        (status = 204, description = "Cleared require re-auth successfully"),
    ),
    params(
        ("authed_token_id" = Uuid, Path, description = "Authed token ID"),
    ),
)]
pub async fn clear_require_reauth_token(
    State(state): State<AppState>,
    Path(authed_token_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    state
        .services
        .authed_token
        .clear_require_reauth(authed_token_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
