use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Response,
    routing::{delete, get, patch, post},
    Json, Router,
};
use eddist_core::domain::notice::Notice;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    repository::notice_repository::{CreateNoticeInput, NoticeRepository, UpdateNoticeInput},
    DefaultAppState,
};

pub fn routes() -> Router<DefaultAppState> {
    Router::new()
        .route("/notices", get(get_notices))
        .route("/notices", post(create_notice))
        .route("/notices/:id", get(get_notice))
        .route("/notices/:id", patch(update_notice))
        .route("/notices/:id", delete(delete_notice))
}

#[derive(Debug, Deserialize)]
pub struct NoticeListQuery {
    #[serde(default)]
    pub page: u32,
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_limit() -> u32 {
    20
}

#[utoipa::path(
    get,
    path = "/notices/",
    responses(
        (status = 200, description = "List notices successfully", body = Vec<Notice>),
    )
)]
pub async fn get_notices(
    State(state): State<DefaultAppState>,
    Query(query): Query<NoticeListQuery>,
) -> Json<Vec<Notice>> {
    let limit = query.limit.min(100);
    let notices = state
        .notice_repo
        .get_notices_paginated(query.page, limit)
        .await
        .unwrap();
    notices.into()
}

#[utoipa::path(
    get,
    path = "/notices/{id}/",
    responses(
        (status = 200, description = "Get notice successfully", body = Notice),
        (status = 404, description = "Notice not found"),
    ),
    params(
        ("id" = Uuid, Path, description = "Notice ID"),
    )
)]
pub async fn get_notice(
    State(state): State<DefaultAppState>,
    Path(id): Path<Uuid>,
) -> Response {
    let notice = state.notice_repo.get_notice_by_id(id).await.unwrap();

    match notice {
        Some(notice) => Response::builder()
            .status(200)
            .body(serde_json::to_string(&notice).unwrap().into())
            .unwrap(),
        None => Response::builder()
            .status(404)
            .body(axum::body::Body::empty())
            .unwrap(),
    }
}

#[utoipa::path(
    post,
    path = "/notices/",
    request_body = CreateNoticeInput,
    responses(
        (status = 201, description = "Notice created successfully", body = Notice),
        (status = 400, description = "Invalid input"),
    )
)]
pub async fn create_notice(
    State(state): State<DefaultAppState>,
    Json(input): Json<CreateNoticeInput>,
) -> Response {
    // TODO: Get author_id from session/auth
    let author_id = None;

    match state.notice_repo.create_notice(input, author_id).await {
        Ok(notice) => Response::builder()
            .status(201)
            .body(serde_json::to_string(&notice).unwrap().into())
            .unwrap(),
        Err(e) => {
            tracing::error!("Failed to create notice: {:?}", e);
            Response::builder()
                .status(400)
                .body(format!("Failed to create notice: {}", e).into())
                .unwrap()
        }
    }
}

#[utoipa::path(
    patch,
    path = "/notices/{id}/",
    request_body = UpdateNoticeInput,
    responses(
        (status = 200, description = "Notice updated successfully", body = Notice),
        (status = 404, description = "Notice not found"),
        (status = 400, description = "Invalid input"),
    ),
    params(
        ("id" = Uuid, Path, description = "Notice ID"),
    )
)]
pub async fn update_notice(
    State(state): State<DefaultAppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateNoticeInput>,
) -> Response {
    match state.notice_repo.update_notice(id, input).await {
        Ok(notice) => Response::builder()
            .status(200)
            .body(serde_json::to_string(&notice).unwrap().into())
            .unwrap(),
        Err(e) => {
            let status = if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::BAD_REQUEST
            };

            Response::builder()
                .status(status)
                .body(format!("Failed to update notice: {}", e).into())
                .unwrap()
        }
    }
}

#[utoipa::path(
    delete,
    path = "/notices/{id}/",
    responses(
        (status = 204, description = "Notice deleted successfully"),
        (status = 404, description = "Notice not found"),
    ),
    params(
        ("id" = Uuid, Path, description = "Notice ID"),
    )
)]
pub async fn delete_notice(
    State(state): State<DefaultAppState>,
    Path(id): Path<Uuid>,
) -> Response {
    match state.notice_repo.delete_notice(id).await {
        Ok(_) => Response::builder()
            .status(204)
            .body(axum::body::Body::empty())
            .unwrap(),
        Err(e) => {
            tracing::error!("Failed to delete notice: {:?}", e);
            Response::builder()
                .status(404)
                .body(format!("Failed to delete notice: {}", e).into())
                .unwrap()
        }
    }
}
