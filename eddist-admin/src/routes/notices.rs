use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, patch, post},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AppState,
    auth::AdminIdentity,
    error::ApiError,
    models::Notice,
    repository::notice_repository::{CreateNoticeInput, UpdateNoticeInput},
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
        .services
        .content_admin
        .get_notices(query.page, limit)
        .await?;
    Ok(Json(notices))
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
        .services
        .content_admin
        .get_notice(id)
        .await?
        .ok_or_else(|| ApiError::not_found("Notice not found"))?;
    Ok(Json(notice))
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
    identity: AdminIdentity,
    Json(input): Json<CreateNoticeInput>,
) -> Result<(StatusCode, Json<Notice>), ApiError> {
    if input.slug == "latest" {
        return Err(ApiError::bad_request("'latest' is a reserved slug"));
    }

    let notice = state
        .services
        .content_admin
        .create_notice(&identity, input)
        .await
        .map_err(|e| ApiError::bad_request(format!("Failed to create notice: {e}")))?;
    Ok((StatusCode::CREATED, Json(notice)))
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
    identity: AdminIdentity,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateNoticeInput>,
) -> Result<Json<Notice>, ApiError> {
    state
        .services
        .content_admin
        .check_notice_author(&identity, id)
        .await
        .map_err(|e| {
            if e.to_string().contains("Forbidden") {
                ApiError::forbidden(e.to_string())
            } else {
                ApiError::not_found(e.to_string())
            }
        })?;

    if matches!(&input.slug, Some(slug) if slug == "latest") {
        return Err(ApiError::bad_request("'latest' is a reserved slug"));
    }

    let notice = state
        .services
        .content_admin
        .update_notice(&identity, id, input)
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                ApiError::not_found("Notice not found")
            } else {
                ApiError::bad_request(format!("Failed to update notice: {e}"))
            }
        })?;
    Ok(Json(notice))
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
    identity: AdminIdentity,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    state
        .services
        .content_admin
        .check_notice_author(&identity, id)
        .await
        .map_err(|e| {
            if e.to_string().contains("Forbidden") {
                ApiError::forbidden(e.to_string())
            } else {
                ApiError::not_found(e.to_string())
            }
        })?;

    state
        .services
        .content_admin
        .delete_notice(&identity, id)
        .await
        .map_err(|e| ApiError::not_found(format!("Failed to delete notice: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}
