use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, patch, post},
};
use uuid::Uuid;

use crate::{
    AppState,
    auth::AdminIdentity,
    error::ApiError,
    models::{Res, Thread, ThreadCompactionInput, UpdateResInput},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/boards/{boardKey}/threads", get(get_threads))
        .route("/boards/{boardKey}/threads/{threadId}", get(get_thread))
        .route(
            "/boards/{boardKey}/threads/{threadId}/responses",
            get(get_responses),
        )
        .route(
            "/boards/{boardKey}/threads/{threadId}/responses/{resId}",
            patch(update_response),
        )
        .route(
            "/boards/{boardKey}/threads-compaction/",
            post(threads_compaction),
        )
}

#[utoipa::path(
    get,
    path = "/boards/{board_key}/threads/",
    responses(
        (status = 200, description = "List threads successfully", body = Vec<Thread>),
    )
)]
pub async fn get_threads(
    State(state): State<AppState>,
    Path(board_key): Path<String>,
) -> Result<Json<Vec<Thread>>, ApiError> {
    let threads = state.services.thread.get_threads(&board_key).await?;
    Ok(Json(threads))
}

#[utoipa::path(
    get,
    path = "/boards/{board_key}/threads/{thread_id}/",
    responses(
        (status = 200, description = "Get thread successfully", body = Thread),
        (status = 404, description = "Thread not found"),
    ),
    params(
        ("board_key" = String, Path, description = "Board ID"),
        ("thread_id" = u64, Path, description = "Thread ID"),
    )
)]
pub async fn get_thread(
    State(state): State<AppState>,
    Path((board_key, thread_id)): Path<(String, u64)>,
) -> Result<Json<Thread>, ApiError> {
    let thread = state
        .services
        .thread
        .get_thread(&board_key, thread_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Thread not found"))?;
    Ok(Json(thread))
}

#[utoipa::path(
    get,
    path = "/boards/{board_key}/threads/{thread_id}/responses/",
    responses(
        (status = 200, description = "List responses successfully", body = Vec<Res>),
        (status = 404, description = "Thread not found"),
    ),
    params(
        ("thread_id" = u64, Path, description = "Thread ID"),
    )
)]
pub async fn get_responses(
    State(state): State<AppState>,
    Path((board_key, thread_id)): Path<(String, u64)>,
) -> Result<Json<Vec<Res>>, ApiError> {
    let responses = state
        .services
        .thread
        .get_responses(&board_key, thread_id)
        .await?;
    Ok(Json(responses))
}

#[utoipa::path(
    patch,
    path = "/boards/{board_key}/threads/{thread_id}/responses/{res_id}/",
    responses(
        (status = 200, description = "Update response successfully", body = Res),
    ),
    params(
        ("board_key" = String, Path, description = "Board ID"),
        ("thread_id" = u64, Path, description = "Thread ID"),
        ("res_id" = Uuid, Path, description = "Response ID"),
    ),
    request_body = UpdateResInput
)]
pub async fn update_response(
    State(state): State<AppState>,
    identity: AdminIdentity,
    Path((board_key, thread_id, res_id)): Path<(String, u64, Uuid)>,
    Json(body): Json<UpdateResInput>,
) -> Result<Json<Res>, ApiError> {
    let res = state
        .services
        .thread
        .update_response(&identity, &board_key, thread_id, res_id, body)
        .await?;
    Ok(Json(res))
}

#[utoipa::path(
    post,
    path = "/boards/{board_key}/threads-compaction/",
    responses(
        (status = 200, description = "Compaction thread successfully"),
    ),
    params(
        ("board_key" = String, Path, description = "Board Key"),
    ),
    request_body = ThreadCompactionInput
)]
pub async fn threads_compaction(
    State(state): State<AppState>,
    identity: AdminIdentity,
    Path(board_key): Path<String>,
    Json(body): Json<ThreadCompactionInput>,
) -> Result<StatusCode, ApiError> {
    state
        .services
        .thread
        .compact_threads(&identity, &board_key, body.target_count)
        .await?;
    Ok(StatusCode::OK)
}
