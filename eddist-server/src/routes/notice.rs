use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{app::AppState, repositories::notice_repository::NoticeRepository};

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

/// Public API response for a single notice (excludes internal fields)
#[derive(Debug, Serialize)]
pub struct NoticeResponse {
    pub slug: String,
    pub title: String,
    pub content: String,
    pub published_at: chrono::NaiveDateTime,
}

impl From<eddist_core::domain::notice::Notice> for NoticeResponse {
    fn from(notice: eddist_core::domain::notice::Notice) -> Self {
        NoticeResponse {
            slug: notice.slug,
            title: notice.title,
            content: notice.content,
            published_at: notice.published_at,
        }
    }
}

/// Public API response for notice list item (excludes internal fields)
#[derive(Debug, Serialize)]
pub struct NoticeListItemResponse {
    pub slug: String,
    pub title: String,
    pub published_at: chrono::NaiveDateTime,
}

impl From<eddist_core::domain::notice::NoticeListItem> for NoticeListItemResponse {
    fn from(item: eddist_core::domain::notice::NoticeListItem) -> Self {
        NoticeListItemResponse {
            slug: item.slug,
            title: item.title,
            published_at: item.published_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct NoticeListResponse {
    pub notices: Vec<NoticeListItemResponse>,
    pub total: i64,
    pub page: u32,
    pub limit: u32,
}

pub async fn get_latest_notices(State(state): State<AppState>) -> impl IntoResponse {
    match state.notice_repo.get_notices_paginated(0, 3).await {
        Ok(notices) => {
            let response = notices
                .into_iter()
                .map(Into::into)
                .collect::<Vec<NoticeListItemResponse>>();
            let mut resp = Json(response).into_response();
            resp.headers_mut()
                .insert("Cache-Control", "s-maxage=300".parse().unwrap());
            resp
        }
        Err(e) => {
            tracing::error!("Failed to get latest notices: {e:?}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
        }
    }
}

pub async fn get_notices_paginated(
    State(state): State<AppState>,
    Query(query): Query<NoticeListQuery>,
) -> impl IntoResponse {
    let limit = query.limit.min(100);
    match tokio::try_join!(
        state.notice_repo.get_notices_paginated(query.page, limit),
        state.notice_repo.count_notices()
    ) {
        Ok((notices, total)) => {
            let response = NoticeListResponse {
                notices: notices.into_iter().map(Into::into).collect(),
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
            tracing::error!("Failed to get notices: {e:?}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
        }
    }
}

pub async fn get_notice_by_slug(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    match state.notice_repo.get_notice_by_slug(&slug).await {
        Ok(Some(notice)) => {
            let response: NoticeResponse = notice.into();
            let mut resp = Json(response).into_response();
            resp.headers_mut()
                .insert("Cache-Control", "s-maxage=3600".parse().unwrap());
            resp
        }
        Ok(None) => (StatusCode::NOT_FOUND, "Notice not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to get notice: {e:?}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
        }
    }
}
