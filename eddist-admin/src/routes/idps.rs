use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, patch, post},
};
use uuid::Uuid;

use crate::{
    AppState,
    auth::AdminIdentity,
    error::ApiError,
    models::idp::{CreateIdpInput, Idp, UpdateIdpInput},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/idps", get(list_idps))
        .route("/idps", post(create_idp))
        .route("/idps/{id}", get(get_idp))
        .route("/idps/{id}", patch(update_idp))
        .route("/idps/{id}", delete(delete_idp))
}

#[utoipa::path(
    get,
    path = "/idps/",
    tag = "idps",
    responses(
        (status = 200, description = "List all IdPs", body = Vec<Idp>),
    )
)]
pub async fn list_idps(State(state): State<AppState>) -> Result<Json<Vec<Idp>>, ApiError> {
    let idps = state.services.content_admin.list_idps().await?;
    Ok(Json(idps))
}

#[utoipa::path(
    get,
    path = "/idps/{id}/",
    tag = "idps",
    responses(
        (status = 200, description = "Get IdP by ID", body = Idp),
        (status = 404, description = "IdP not found"),
    ),
    params(
        ("id" = Uuid, Path, description = "IdP ID"),
    )
)]
pub async fn get_idp(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Idp>, ApiError> {
    let idp = state
        .services
        .content_admin
        .get_idp(id)
        .await?
        .ok_or_else(|| ApiError::not_found("IdP not found"))?;
    Ok(Json(idp))
}

#[utoipa::path(
    post,
    path = "/idps/",
    tag = "idps",
    request_body = CreateIdpInput,
    responses(
        (status = 201, description = "IdP created successfully", body = Idp),
        (status = 400, description = "Invalid input"),
    )
)]
pub async fn create_idp(
    State(state): State<AppState>,
    identity: AdminIdentity,
    Json(input): Json<CreateIdpInput>,
) -> Result<(StatusCode, Json<Idp>), ApiError> {
    let idp = state
        .services
        .content_admin
        .create_idp(&identity, input)
        .await?;
    Ok((StatusCode::CREATED, Json(idp)))
}

#[utoipa::path(
    patch,
    path = "/idps/{id}/",
    tag = "idps",
    request_body = UpdateIdpInput,
    responses(
        (status = 200, description = "IdP updated successfully", body = Idp),
        (status = 404, description = "IdP not found"),
        (status = 400, description = "Invalid input"),
    ),
    params(
        ("id" = Uuid, Path, description = "IdP ID"),
    )
)]
pub async fn update_idp(
    State(state): State<AppState>,
    identity: AdminIdentity,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateIdpInput>,
) -> Result<Json<Idp>, ApiError> {
    let idp = state
        .services
        .content_admin
        .update_idp(&identity, id, input)
        .await?;
    Ok(Json(idp))
}

#[utoipa::path(
    delete,
    path = "/idps/{id}/",
    tag = "idps",
    responses(
        (status = 204, description = "IdP deleted successfully"),
        (status = 404, description = "IdP not found"),
    ),
    params(
        ("id" = Uuid, Path, description = "IdP ID"),
    )
)]
pub async fn delete_idp(
    State(state): State<AppState>,
    identity: AdminIdentity,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    state
        .services
        .content_admin
        .delete_idp(&identity, id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
