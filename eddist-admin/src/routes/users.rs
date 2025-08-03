use axum::{
    extract::{Path, Query, State},
    response::Response,
    routing::{get, patch},
    Json, Router,
};
use uuid::Uuid;

use crate::{
    models::{User, UserSearchQuery, UserStatusUpdateInput},
    repository::admin_user_repository::AdminUserRepository,
    DefaultAppState,
};

pub fn routes() -> Router<DefaultAppState> {
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
    State(state): State<DefaultAppState>,
    Query(query): Query<UserSearchQuery>,
) -> Json<Vec<User>> {
    let users = state
        .user_repo
        .search_users(query.user_id, query.user_name, query.authed_token_id)
        .await
        .unwrap();

    Json(users)
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
    State(state): State<DefaultAppState>,
    Path(user_id): Path<Uuid>,
    Json(body): Json<UserStatusUpdateInput>,
) -> Response {
    state
        .user_repo
        .update_user_status(user_id, body.enabled)
        .await
        .unwrap();

    let users = state
        .user_repo
        .search_users(Some(user_id), None, None)
        .await
        .unwrap();

    Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&users[0]).unwrap().into())
        .unwrap()
}
