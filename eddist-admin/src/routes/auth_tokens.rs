use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get},
};
use eddist_core::{
    domain::pubsub_repository::{AuthTokenRevoked, CHANNEL_AUTH_TOKEN_REVOKED},
    utils::is_authed_token_backup_enabled,
};
use redis::AsyncCommands;
use tracing::warn;
use uuid::Uuid;

use crate::{
    AppState,
    error::ApiError,
    models::{DeleteAuthedTokenInput, ListAuthedTokensQuery, PaginatedAuthedTokens},
    repository::authed_token_repository::ListAuthedTokensParams,
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
        .list_authed_tokens(ListAuthedTokensParams {
            offset,
            limit: per_page,
            origin_ip: query.origin_ip.as_deref(),
            writing_ua: query.writing_ua.as_deref(),
            authed_ua: query.authed_ua.as_deref(),
            asn_num: query.asn_num,
            validity: query.validity,
            sort_column,
            sort_asc,
        })
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
    let affected_ids = if !using_origin_ip {
        state
            .authed_token_repo
            .delete_authed_token(authed_token_id)
            .await?;
        vec![authed_token_id]
    } else {
        state
            .authed_token_repo
            .delete_authed_token_by_origin_ip(authed_token_id)
            .await?
    };
    if is_authed_token_backup_enabled() {
        let mut conn = state.redis_conn.clone();
        for id in affected_ids {
            publish_token_revoked(&mut conn, id).await;
        }
    }
    Ok(StatusCode::OK)
}

async fn publish_token_revoked(conn: &mut redis::aio::ConnectionManager, authed_token_id: Uuid) {
    match serde_json::to_string(&AuthTokenRevoked { authed_token_id }) {
        Ok(payload) => {
            let _: Result<(), _> = conn.publish(CHANNEL_AUTH_TOKEN_REVOKED, payload).await;
        }
        Err(e) => {
            warn!("Failed to serialize AuthTokenRevoked for {authed_token_id}: {e}");
        }
    }
}
