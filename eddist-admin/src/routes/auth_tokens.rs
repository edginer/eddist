use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get},
    Json, Router,
};
use uuid::Uuid;

use crate::{
    error::ApiError,
    models::{DeleteAuthedTokenInput, ListAuthedTokensQuery, PaginatedAuthedTokens},
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/authed_tokens", get(list_authed_tokens))
        .route("/authed_tokens/{authedTokenId}", get(get_authed_token))
        .route(
            "/authed_tokens/{authedTokenId}",
            delete(delete_authed_token),
        )
}

const ALLOWED_SORT_COLUMNS: &[&str] = &["created_at", "authed_at", "last_wrote_at"];

#[utoipa::path(
    get,
    path = "/authed_tokens",
    responses(
        (status = 200, description = "List authed tokens successfully", body = PaginatedAuthedTokens),
    ),
    params(
        ListAuthedTokensQuery,
    ),
)]
pub async fn list_authed_tokens(
    State(state): State<AppState>,
    Query(query): Query<ListAuthedTokensQuery>,
) -> Result<Json<PaginatedAuthedTokens>, ApiError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(50).clamp(1, 100);
    let offset = (page - 1) as u64 * per_page as u64;

    let sort_column = query
        .sort_by
        .as_deref()
        .filter(|s| ALLOWED_SORT_COLUMNS.contains(s))
        .unwrap_or("created_at");
    let sort_asc = query
        .sort_order
        .as_deref()
        .map(|s| s == "asc")
        .unwrap_or(false);

    let (items, total) = state
        .authed_token_repo
        .list_authed_tokens(
            offset,
            per_page,
            query.origin_ip.as_deref(),
            query.writing_ua.as_deref(),
            query.authed_ua.as_deref(),
            query.asn_num,
            query.validity,
            sort_column,
            sort_asc,
        )
        .await?;

    let total_pages = ((total as f64) / (per_page as f64)).ceil() as u32;

    Ok(Json(PaginatedAuthedTokens {
        items,
        total,
        page,
        per_page,
        total_pages,
    }))
}

#[utoipa::path(
    get,
    path = "/authed_tokens/{authed_token_id}/",
    responses(
        (status = 200, description = "Get authed token successfully", body = crate::models::AuthedToken),
    ),
    params(
        ("authed_token_id" = Uuid, Path, description = "Authed token ID"),
    ),
)]
pub async fn get_authed_token(
    State(state): State<AppState>,
    Path(authed_token_id): Path<Uuid>,
) -> Result<Json<crate::models::AuthedToken>, ApiError> {
    let authed_token = state
        .authed_token_repo
        .get_authed_token(authed_token_id)
        .await?;
    Ok(Json(authed_token))
}

#[utoipa::path(
    delete,
    path = "/authed_tokens/{authed_token_id}/",
    responses(
        (status = 200, description = "Delete authed token successfully"),
    ),
    params(
        ("authed_token_id" = Uuid, Path, description = "Authed token ID"),
        DeleteAuthedTokenInput
    ),
)]
pub async fn delete_authed_token(
    State(state): State<AppState>,
    Path(authed_token_id): Path<Uuid>,
    Query(DeleteAuthedTokenInput { using_origin_ip }): Query<DeleteAuthedTokenInput>,
) -> Result<StatusCode, ApiError> {
    if !using_origin_ip {
        state
            .authed_token_repo
            .delete_authed_token(authed_token_id)
            .await?;
    } else {
        state
            .authed_token_repo
            .delete_authed_token_by_origin_ip(authed_token_id)
            .await?;
    }
    Ok(StatusCode::OK)
}
