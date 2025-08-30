use axum::{
    extract::{Path, Query, State},
    response::Response,
    routing::{delete, get, patch},
    Json, Router,
};
use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};
use utoipa::IntoParams;

use crate::{
    models::{Res, Thread},
    repository::{
        admin_archive_repository::{AdminArchiveRepository, ArchivedResUpdate},
        admin_bbs_repository::AdminBbsRepository,
    },
    DefaultAppState,
};

pub fn routes() -> Router<DefaultAppState> {
    Router::new()
        .route("/boards/{boardKey}/archives", get(get_archived_threads))
        .route(
            "/boards/{boardKey}/archives/{threadId}",
            get(get_archived_thread),
        )
        .route(
            "/boards/{boardKey}/archives/{threadId}/responses",
            get(get_archived_responses),
        )
        .route(
            "/boards/{boardKey}/dat-archives/{threadNumber}",
            get(get_dat_archived_thread),
        )
        .route(
            "/boards/{boardKey}/admin-dat-archives/{threadNumber}",
            get(get_admin_dat_archived_thread),
        )
        .route(
            "/boards/{boardKey}/dat-archives/{threadNumber}/responses",
            patch(update_archived_res),
        )
        .route(
            "/boards/{boardKey}/dat-archives/{threadNumber}/responses/{resOrder}",
            delete(delete_archived_res),
        )
        .route(
            "/boards/{boardKey}/dat-archives/{threadNumber}",
            delete(delete_archived_thread),
        )
}

#[derive(Debug, Clone, Serialize, Deserialize, IntoParams)]
pub struct GetArchivedThreadsQuery {
    keyword: Option<String>,
    start: Option<u64>,
    end: Option<u64>,
    page: Option<u64>,
    limit: Option<u64>,
}

#[utoipa::path(
    get,
    path = "/boards/{board_key}/archives/",
    params(
        ("board_key" = String, Path, description = "Board ID"),
        GetArchivedThreadsQuery
    ),
    responses(
        (status = 200, description = "List threads successfully", body = Vec<Thread>),
    )
)]
pub async fn get_archived_threads(
    State(state): State<DefaultAppState>,
    Path(board_key): Path<String>,
    Query(GetArchivedThreadsQuery {
        keyword,
        start,
        end,
        page,
        limit,
    }): Query<GetArchivedThreadsQuery>,
) -> Json<Vec<Thread>> {
    let threads = state
        .admin_bbs_repo
        .get_archived_threads_by_filter(
            &board_key,
            keyword.as_deref(),
            (
                start.map(|x| Utc.timestamp_opt(x as i64, 0).unwrap().to_utc()),
                end.map(|x| Utc.timestamp_opt(x as i64, 0).unwrap().to_utc()),
            ),
            page.unwrap_or(0),
            limit.unwrap_or(20),
        )
        .await
        .unwrap();

    threads.into()
}

#[utoipa::path(
    get,
    path = "/boards/{board_key}/archives/{thread_id}/",
    responses(
        (status = 200, description = "Get thread successfully", body = Thread),
        (status = 404, description = "Thread not found"),
    ),
    params(
        ("board_key" = String, Path, description = "Board ID"),
        ("thread_id" = u64, Path, description = "Thread ID"),
    )
)]
pub async fn get_archived_thread(
    State(state): State<DefaultAppState>,
    Path((board_key, thread_id)): Path<(String, u64)>,
) -> Response {
    let thread = state
        .admin_bbs_repo
        .get_archived_threads_by_thread_id(&board_key, Some(vec![thread_id]))
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
    path = "/boards/{board_key}/archives/{thread_id}/responses/",
    responses(
        (status = 200, description = "List responses successfully", body = Vec<Res>),
        (status = 404, description = "Thread not found"),
    ),
    params(
        ("thread_id" = u64, Path, description = "Thread ID"),
    )
)]
pub async fn get_archived_responses(
    State(state): State<DefaultAppState>,
    Path((board_key, thread_id)): Path<(String, u64)>,
) -> Json<Vec<Res>> {
    let responses = state
        .admin_bbs_repo
        .get_archived_reses_by_thread_id(&board_key, thread_id)
        .await
        .unwrap();

    responses.into()
}

#[utoipa::path(
    get,
    path = "/boards/{board_key}/dat-archives/{thread_number}/",
    responses(
        (status = 200, description = "Get archived thread successfully", body = crate::repository::admin_archive_repository::ArchivedThread),
    ),
    params(
        ("board_key" = String, Path, description = "Board ID"),
        ("thread_number" = u64, Path, description = "Thread ID"),
    )
)]
pub async fn get_dat_archived_thread(
    State(state): State<DefaultAppState>,
    Path((board_key, thread_number)): Path<(String, u64)>,
) -> Response {
    match state
        .admin_archive_repo
        .get_thread(&board_key, thread_number)
        .await
    {
        Ok(thread) => Response::builder()
            .status(200)
            .body(serde_json::to_string(&thread).unwrap().into())
            .unwrap(),
        Err(_) => Response::builder()
            .status(500)
            .body(axum::body::Body::empty())
            .unwrap(),
    }
}

#[utoipa::path(
    get,
    path = "/boards/{board_key}/admin-dat-archives/{thread_number}/",
    responses(
        (status = 200, description = "Get archived thread successfully", body = crate::repository::admin_archive_repository::ArchivedAdminThread),
    ),
    params(
        ("board_key" = String, Path, description = "Board ID"),
        ("thread_number" = u64, Path, description = "Thread ID"),
    )
)]
pub async fn get_admin_dat_archived_thread(
    State(state): State<DefaultAppState>,
    Path((board_key, thread_number)): Path<(String, u64)>,
) -> Response {
    match state
        .admin_archive_repo
        .get_archived_admin_thread(&board_key, thread_number)
        .await
    {
        Ok(thread) => Response::builder()
            .status(200)
            .body(serde_json::to_string(&thread).unwrap().into())
            .unwrap(),
        Err(_) => Response::builder()
            .status(500)
            .body(axum::body::Body::empty())
            .unwrap(),
    }
}

#[utoipa::path(
    patch,
    path = "/boards/{board_key}/dat-archives/{thread_number}/responses/",
    responses(
        (status = 200, description = "Update archived response successfully", body = ()),
    ),
    params(
        ("board_key" = String, Path, description = "Board ID"),
        ("thread_number" = u64, Path, description = "Thread ID"),
    ),
    request_body = Vec<ArchivedResUpdate>,
)]
pub async fn update_archived_res(
    State(state): State<DefaultAppState>,
    Path((board_key, thread_number)): Path<(String, u64)>,
    Json(body): Json<Vec<ArchivedResUpdate>>,
) -> Response {
    if let Err(e) = state
        .admin_archive_repo
        .update_response(&board_key, thread_number, &body)
        .await
    {
        Response::builder()
            .status(500)
            .body(e.to_string().into())
            .unwrap()
    } else {
        Response::builder()
            .status(200)
            .body(axum::body::Body::empty())
            .unwrap()
    }
}

#[utoipa::path(
    delete,
    path = "/boards/{board_key}/dat-archives/{thread_number}/responses/{res_order}/",
    responses(
        (status = 200, description = "Delete response successfully"),
    ),
    params(
        ("board_key" = String, Path, description = "Board ID"),
        ("thread_number" = u64, Path, description = "Thread ID"),
        ("res_order" = u64, Path, description = "Response order"),
    ),
)]
pub async fn delete_archived_res(
    State(state): State<DefaultAppState>,
    Path((board_key, thread_number, res_order)): Path<(String, u64, u64)>,
) -> Response {
    if let Err(e) = state
        .admin_archive_repo
        .delete_response(&board_key, thread_number, res_order)
        .await
    {
        Response::builder()
            .status(500)
            .body(e.to_string().into())
            .unwrap()
    } else {
        Response::builder()
            .status(200)
            .body(axum::body::Body::empty())
            .unwrap()
    }
}

#[utoipa::path(
    delete,
    path = "/boards/{board_key}/dat-archives/{thread_number}/",
    responses(
        (status = 200, description = "Delete thread successfully"),
    ),
    params(
        ("board_key" = String, Path, description = "Board ID"),
        ("thread_number" = u64, Path, description = "Thread ID"),
    ),
)]
pub async fn delete_archived_thread(
    State(state): State<DefaultAppState>,
    Path((board_key, thread_number)): Path<(String, u64)>,
) -> Response {
    if let Err(e) = state
        .admin_archive_repo
        .delete_thread(&board_key, thread_number)
        .await
    {
        Response::builder()
            .status(500)
            .body(e.to_string().into())
            .unwrap()
    } else {
        Response::builder()
            .status(200)
            .body(axum::body::Body::empty())
            .unwrap()
    }
}
