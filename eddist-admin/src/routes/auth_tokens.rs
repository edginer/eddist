use axum::{
    extract::{Path, Query, State},
    response::Response,
    routing::{delete, get},
    Router,
};
use uuid::Uuid;

use crate::{
    models::{AuthedToken, DeleteAuthedTokenInput, ListAuthedTokensQuery, PaginatedAuthedTokens},
    repository::authed_token_repository::AuthedTokenRepository,
    DefaultAppState,
};

pub fn routes() -> Router<DefaultAppState> {
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
    State(state): State<DefaultAppState>,
    Query(query): Query<ListAuthedTokensQuery>,
) -> Response {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(50).min(100).max(1);
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
        .await
        .unwrap();

    let total_pages = ((total as f64) / (per_page as f64)).ceil() as u32;

    let result = PaginatedAuthedTokens {
        items,
        total,
        page,
        per_page,
        total_pages,
    };

    Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(serde_json::to_string(&result).unwrap().into())
        .unwrap()
}

#[utoipa::path(
    get,
    path = "/authed_tokens/{authed_token_id}/",
    responses(
        (status = 200, description = "Get authed token successfully", body = AuthedToken),
    ),
    params(
        ("authed_token_id" = Uuid, Path, description = "Authed token ID"),
    ),
)]
pub async fn get_authed_token(
    State(state): State<DefaultAppState>,
    Path(authed_token_id): Path<Uuid>,
) -> Response {
    let authed_token = state
        .authed_token_repo
        .get_authed_token(authed_token_id)
        .await
        .unwrap();

    Response::builder()
        .status(200)
        .body(serde_json::to_string(&authed_token).unwrap().into())
        .unwrap()
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
    State(state): State<DefaultAppState>,
    Path(authed_token_id): Path<Uuid>,
    Query(DeleteAuthedTokenInput { using_origin_ip }): Query<DeleteAuthedTokenInput>,
) -> Response {
    if !using_origin_ip {
        state
            .authed_token_repo
            .delete_authed_token(authed_token_id)
            .await
            .unwrap();
    } else {
        state
            .authed_token_repo
            .delete_authed_token_by_origin_ip(authed_token_id)
            .await
            .unwrap();
    }

    Response::builder()
        .status(200)
        .body(axum::body::Body::empty())
        .unwrap()
}
