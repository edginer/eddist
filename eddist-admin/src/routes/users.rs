use axum::{
    extract::{Path, Query, State},
    routing::{get, patch},
    Json, Router,
};
use uuid::Uuid;

use crate::{
    error::ApiError,
    models::{User, UserSearchQuery, UserStatusUpdateInput},
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/users/search", get(search_users))
        .route("/users/{userId}/status", patch(update_user_status))
}

#[utoipa::path(
    get,
    path = "/users/search/",
    params(
        UserSearchQuery
    ),
    responses(
        (status = 200, description = "List users successfully", body = Vec<User>),
    )
)]
pub async fn search_users(
    State(state): State<AppState>,
    Query(query): Query<UserSearchQuery>,
) -> Result<Json<Vec<User>>, ApiError> {
    let users = state
        .user_repo
        .search_users(query.user_id, query.user_name, query.authed_token_id)
        .await?;
    Ok(Json(users))
}

#[utoipa::path(
    patch,
    path = "/users/{user_id}/status/",
    responses(
        (status = 200, description = "Update user status successfully", body = User),
    ),
    params(
        ("user_id" = Uuid, Path, description = "User ID"),
    ),
    request_body = UserStatusUpdateInput
)]
pub async fn update_user_status(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(body): Json<UserStatusUpdateInput>,
) -> Result<Json<User>, ApiError> {
    state
        .user_repo
        .update_user_status(user_id, body.enabled)
        .await?;

    let users = state
        .user_repo
        .search_users(Some(user_id), None, None)
        .await?;

    let user = users
        .into_iter()
        .next()
        .ok_or_else(|| ApiError::not_found("User not found after update"))?;
    Ok(Json(user))
}
