use axum::{
    extract::{Path, Query, State},
    response::Response,
    routing::{delete, get},
    Router,
};
use uuid::Uuid;

use crate::{
    models::{AuthedToken, DeleteAuthedTokenInput},
    repository::authed_token_repository::AuthedTokenRepository,
    DefaultAppState,
};

pub fn routes() -> Router<DefaultAppState> {
    Router::new()
        .route("/authed_tokens/{authedTokenId}", get(get_authed_token))
        .route(
            "/authed_tokens/{authedTokenId}",
            delete(delete_authed_token),
        )
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
