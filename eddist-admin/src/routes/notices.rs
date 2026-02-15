use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, patch, post},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    auth::{AdminEmail, AdminSession},
    error::ApiError,
    models::Notice,
    repository::notice_repository::{CreateNoticeInput, UpdateNoticeInput},
    AppState,
};

pub fn routes() -> Router<AppState> {
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
    state: &AppState,
    admin_session: &AdminSession,
    notice_id: Uuid,
) -> Result<eddist_core::domain::notice::Notice, ApiError> {
    let current_admin_email = admin_session
        .get_admin_email()
        .ok_or_else(ApiError::unauthorized)?;

    let existing_notice = state
        .notice_repo
        .get_notice_by_id(notice_id)
        .await?
        .ok_or_else(|| ApiError::not_found("Notice not found"))?;

    if existing_notice.author_email.as_ref() != Some(&current_admin_email) {
        return Err(ApiError::forbidden(
            "You can only modify notices you created",
        ));
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
    State(state): State<AppState>,
    Query(query): Query<NoticeListQuery>,
) -> Result<Json<Vec<Notice>>, ApiError> {
    let limit = query.limit.min(100);
    let notices = state
        .notice_repo
        .get_notices_paginated(query.page, limit)
        .await?;
    Ok(Json(notices.into_iter().map(Notice::from).collect()))
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
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Notice>, ApiError> {
    let notice = state
        .notice_repo
        .get_notice_by_id(id)
        .await?
        .ok_or_else(|| ApiError::not_found("Notice not found"))?;
    Ok(Json(notice.into()))
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
    State(state): State<AppState>,
    AdminEmail(email): AdminEmail,
    Json(input): Json<CreateNoticeInput>,
) -> Result<(StatusCode, Json<Notice>), ApiError> {
    if input.slug == "latest" {
        return Err(ApiError::bad_request("'latest' is a reserved slug"));
    }

    let notice = state
        .notice_repo
        .create_notice(input, Some(email))
        .await
        .map_err(|e| ApiError::bad_request(format!("Failed to create notice: {e}")))?;
    Ok((StatusCode::CREATED, Json(notice.into())))
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
    State(state): State<AppState>,
    admin_session: AdminSession,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateNoticeInput>,
) -> Result<Json<Notice>, ApiError> {
    check_notice_author(&state, &admin_session, id).await?;

    if matches!(&input.slug, Some(slug) if slug == "latest") {
        return Err(ApiError::bad_request("'latest' is a reserved slug"));
    }

    let notice = state
        .notice_repo
        .update_notice(id, input)
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                ApiError::not_found("Notice not found")
            } else {
                ApiError::bad_request(format!("Failed to update notice: {e}"))
            }
        })?;
    Ok(Json(notice.into()))
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
    State(state): State<AppState>,
    admin_session: AdminSession,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    check_notice_author(&state, &admin_session, id).await?;

    state
        .notice_repo
        .delete_notice(id)
        .await
        .map_err(|e| ApiError::not_found(format!("Failed to delete notice: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}
