use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Response,
    routing::{delete, get, patch, post},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    auth::AdminSession,
    models::Notice,
    repository::notice_repository::{CreateNoticeInput, NoticeRepository, UpdateNoticeInput},
    DefaultAppState,
};

pub fn routes() -> Router<DefaultAppState> {
    Router::new()
        .route("/notices", get(get_notices))
        .route("/notices", post(create_notice))
        .route("/notices/{id}", get(get_notice))
        .route("/notices/{id}", patch(update_notice))
        .route("/notices/{id}", delete(delete_notice))
}

#[derive(Debug, Deserialize)]
pub struct NoticeListQuery {
    #[serde(default)]
    pub page: u32,
    #[serde(default = "default_limit")]
    pub limit: u32,
}

const fn default_limit() -> u32 {
    20
}

/// Helper function to check if the current admin is the author of a notice
async fn check_notice_author(
    state: &DefaultAppState,
    admin_session: &AdminSession,
    notice_id: Uuid,
) -> Result<eddist_core::domain::notice::Notice, Response> {
    let Some(current_admin_email) = admin_session.get_admin_email() else {
        return Err(Response::builder()
            .status(401)
            .body("Unauthorized: No user information available".into())
            .unwrap());
    };

    // Get the existing notice to check authorship
    let existing_notice = match state.notice_repo.get_notice_by_id(notice_id).await {
        Ok(Some(notice)) => notice,
        Ok(None) => {
            return Err(Response::builder()
                .status(404)
                .body("Notice not found".into())
                .unwrap());
        }
        Err(e) => {
            tracing::error!("Failed to get notice: {e:?}");
            return Err(Response::builder()
                .status(500)
                .body("Internal server error".into())
                .unwrap());
        }
    };

    // Check if the current admin is the author
    if existing_notice.author_email.as_ref() != Some(&current_admin_email) {
        return Err(Response::builder()
            .status(403)
            .body("Forbidden: You can only modify notices you created".into())
            .unwrap());
    }

    Ok(existing_notice)
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
    Json(notices.into_iter().map(Notice::from).collect())
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
pub async fn get_notice(State(state): State<DefaultAppState>, Path(id): Path<Uuid>) -> Response {
    let notice = state.notice_repo.get_notice_by_id(id).await.unwrap();

    match notice {
        Some(notice) => {
            let admin_notice: Notice = notice.into();
            Response::builder()
                .status(200)
                .body(serde_json::to_string(&admin_notice).unwrap().into())
                .unwrap()
        }
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
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn create_notice(
    State(state): State<DefaultAppState>,
    admin_session: AdminSession,
    Json(input): Json<CreateNoticeInput>,
) -> Response {
    let author_email = admin_session.get_admin_email();

    if author_email.is_none() {
        return Response::builder()
            .status(401)
            .body("Unauthorized: No user information available".into())
            .unwrap();
    }

    if input.slug == "latest" {
        return Response::builder()
            .status(400)
            .body("Invalid input: 'latest' is a reserved slug".into())
            .unwrap();
    }

    match state.notice_repo.create_notice(input, author_email).await {
        Ok(notice) => {
            let admin_notice: Notice = notice.into();
            Response::builder()
                .status(201)
                .body(serde_json::to_string(&admin_notice).unwrap().into())
                .unwrap()
        }
        Err(e) => {
            tracing::error!("Failed to create notice: {e:?}");
            Response::builder()
                .status(400)
                .body(format!("Failed to create notice: {e}").into())
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
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not the notice author"),
    ),
    params(
        ("id" = Uuid, Path, description = "Notice ID"),
    )
)]
pub async fn update_notice(
    State(state): State<DefaultAppState>,
    admin_session: AdminSession,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateNoticeInput>,
) -> Response {
    // Check authorization
    if let Err(response) = check_notice_author(&state, &admin_session, id).await {
        return response;
    }

    if matches!(&input.slug, Some(slug) if slug == "latest") {
        return Response::builder()
            .status(400)
            .body("Invalid input: 'latest' is a reserved slug".into())
            .unwrap();
    }

    match state.notice_repo.update_notice(id, input).await {
        Ok(notice) => {
            let admin_notice: Notice = notice.into();
            Response::builder()
                .status(200)
                .body(serde_json::to_string(&admin_notice).unwrap().into())
                .unwrap()
        }
        Err(e) => {
            let status = if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::BAD_REQUEST
            };

            Response::builder()
                .status(status)
                .body(format!("Failed to update notice: {e}").into())
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
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - not the notice author"),
    ),
    params(
        ("id" = Uuid, Path, description = "Notice ID"),
    )
)]
pub async fn delete_notice(
    State(state): State<DefaultAppState>,
    admin_session: AdminSession,
    Path(id): Path<Uuid>,
) -> Response {
    // Check authorization
    if let Err(response) = check_notice_author(&state, &admin_session, id).await {
        return response;
    }

    match state.notice_repo.delete_notice(id).await {
        Ok(_) => Response::builder()
            .status(204)
            .body(axum::body::Body::empty())
            .unwrap(),
        Err(e) => {
            tracing::error!("Failed to delete notice: {e:?}");
            Response::builder()
                .status(404)
                .body(format!("Failed to delete notice: {e}").into())
                .unwrap()
        }
    }
}
