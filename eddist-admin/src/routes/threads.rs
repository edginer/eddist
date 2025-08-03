use axum::{
    extract::{Path, State},
    response::Response,
    routing::{get, patch, post},
    Json, Router,
};
use eddist_core::domain::res::ResView;
use uuid::Uuid;

use crate::{
    models::{Res, Thread, ThreadCompactionInput, UpdateResInput},
    repository::admin_bbs_repository::AdminBbsRepository,
    DefaultAppState,
};

pub fn routes() -> Router<DefaultAppState> {
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
    State(state): State<DefaultAppState>,
    Path(board_key): Path<String>,
) -> Json<Vec<Thread>> {
    let threads = state
        .admin_bbs_repo
        .get_threads_by_thread_id(&board_key, None)
        .await
        .unwrap();

    threads.into()
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
    State(state): State<DefaultAppState>,
    Path((board_key, thread_id)): Path<(String, u64)>,
) -> Response {
    let thread = state
        .admin_bbs_repo
        .get_threads_by_thread_id(&board_key, Some(vec![thread_id]))
        .await
        .unwrap();

    let Some(thread) = thread.first() else {
        return Response::builder()
            .status(404)
            .body(axum::body::Body::empty())
            .unwrap();
    };

    Response::builder()
        .status(200)
        .body(serde_json::to_string(&thread).unwrap().into())
        .unwrap()
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
    State(state): State<DefaultAppState>,
    Path((board_key, thread_id)): Path<(String, u64)>,
) -> Json<Vec<Res>> {
    let responses = state
        .admin_bbs_repo
        .get_reses_by_thread_id(&board_key, thread_id)
        .await
        .unwrap();

    responses.into()
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
    State(state): State<DefaultAppState>,
    Path((_a, _aa, res_id)): Path<(String, u64, Uuid)>,
    Json(body): Json<UpdateResInput>,
) -> Response {
    let (res, default_name, board_key, thread_number, thread_title) =
        state.admin_bbs_repo.get_res(res_id).await.unwrap();
    let updated_res = state
        .admin_bbs_repo
        .update_res(
            res_id,
            body.author_name.clone(),
            body.mail.clone(),
            body.body.clone(),
            body.is_abone,
        )
        .await
        .unwrap();
    let author_name = if let Some(author_name) = body.author_name {
        author_name
    } else {
        res.author_name.unwrap_or(default_name.clone())
    };
    let mail = if let Some(mail) = body.mail {
        mail
    } else {
        res.mail.unwrap_or_default()
    };
    let is_abone = if let Some(is_abone) = body.is_abone {
        is_abone
    } else {
        res.is_abone
    };
    let res_body = if let Some(body) = body.body {
        body
    } else {
        res.body
    };

    let res_view = ResView {
        author_name,
        mail,
        body: res_body,
        created_at: res.created_at,
        author_id: res.author_id,
        is_abone,
    };

    let res_view = res_view.get_sjis_bytes(&default_name, thread_title.as_deref());
    let mut conn = state.redis_conn;
    let _ = conn
        .send_packed_command(&redis::Cmd::lset(
            format!("threads:{}:{}", board_key, thread_number),
            res.res_order as isize - 1,
            res_view.get_inner(),
        ))
        .await;

    Response::builder()
        .status(200)
        .body(serde_json::to_string(&updated_res).unwrap().into())
        .unwrap()
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
    State(state): State<DefaultAppState>,
    Path(board_key): Path<String>,
    Json(body): Json<ThreadCompactionInput>,
) -> Response {
    state
        .admin_bbs_repo
        .compact_threads(&board_key, body.target_count)
        .await
        .unwrap();

    Response::builder()
        .status(200)
        .body(axum::body::Body::empty())
        .unwrap()
}
