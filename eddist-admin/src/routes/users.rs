use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, patch},
};
use uuid::Uuid;

use crate::{
    AppState,
    auth::AdminIdentity,
    error::ApiError,
    models::{User, UserSearchQuery, UserStatusUpdateInput},
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
    let users = state.services.user.search_users(query).await?;
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
    identity: AdminIdentity,
    Path(user_id): Path<Uuid>,
    Json(body): Json<UserStatusUpdateInput>,
) -> Result<Json<User>, ApiError> {
    let user = state
        .services
        .user
        .update_user_status(&identity, user_id, body.enabled)
        .await?;
    Ok(Json(user))
}
