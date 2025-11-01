use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{repositories::notice_repository::NoticeRepository, AppState};

#[derive(Debug, Deserialize)]
pub struct NoticeListQuery {
    #[serde(default)]
    pub page: u32,
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_limit() -> u32 {
    10
}

#[derive(Debug, Serialize)]
pub struct NoticeListResponse {
    pub notices: Vec<eddist_core::domain::notice::NoticeListItem>,
    pub total: i64,
    pub page: u32,
    pub limit: u32,
}

pub async fn get_latest_notices<R: NoticeRepository>(
    State(notice_repo): State<std::sync::Arc<R>>,
) -> impl IntoResponse {
    match notice_repo.get_latest_notices(3).await {
        Ok(notices) => {
            let mut resp = Json(notices).into_response();
            resp.headers_mut()
                .insert("Cache-Control", "s-maxage=300".parse().unwrap());
            resp
        }
        Err(e) => {
            tracing::error!("Failed to get latest notices: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
        }
    }
}

pub async fn get_notices_paginated<R: NoticeRepository>(
    State(notice_repo): State<std::sync::Arc<R>>,
    Query(query): Query<NoticeListQuery>,
) -> impl IntoResponse {
    let limit = query.limit.min(100); // Cap at 100

    match tokio::try_join!(
        notice_repo.get_notices_paginated(query.page, limit),
        notice_repo.count_notices()
    ) {
        Ok((notices, total)) => {
            let response = NoticeListResponse {
                notices,
                total,
                page: query.page,
                limit,
            };
            let mut resp = Json(response).into_response();
            resp.headers_mut()
                .insert("Cache-Control", "s-maxage=300".parse().unwrap());
            resp
        }
        Err(e) => {
            tracing::error!("Failed to get notices: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
        }
    }
}

pub async fn get_notice_by_id<R: NoticeRepository>(
    State(notice_repo): State<std::sync::Arc<R>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match notice_repo.get_notice_by_id(id).await {
        Ok(Some(notice)) => {
            let mut resp = Json(notice).into_response();
            resp.headers_mut()
                .insert("Cache-Control", "s-maxage=300".parse().unwrap());
            resp
        }
        Ok(None) => (StatusCode::NOT_FOUND, "Notice not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to get notice: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
        }
    }
}
