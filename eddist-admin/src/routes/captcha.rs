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
    models::{CaptchaConfig, CreateCaptchaConfigInput, UpdateCaptchaConfigInput},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/captcha-configs", get(list_captcha_configs))
        .route("/captcha-configs", post(create_captcha_config))
        .route("/captcha-configs/{id}", get(get_captcha_config))
        .route("/captcha-configs/{id}", patch(update_captcha_config))
        .route("/captcha-configs/{id}", delete(delete_captcha_config))
}

#[utoipa::path(
    get,
    path = "/captcha-configs/",
    tag = "captcha",
    responses(
        (status = 200, description = "List all captcha configs successfully", body = Vec<CaptchaConfig>),
    )
)]
pub async fn list_captcha_configs(
    State(state): State<AppState>,
) -> Result<Json<Vec<CaptchaConfig>>, ApiError> {
    let configs = state.services.content_admin.list_captcha_configs().await?;
    Ok(Json(configs))
}

#[utoipa::path(
    get,
    path = "/captcha-configs/{id}/",
    tag = "captcha",
    responses(
        (status = 200, description = "Get captcha config successfully", body = CaptchaConfig),
        (status = 404, description = "Captcha config not found"),
    ),
    params(
        ("id" = Uuid, Path, description = "Captcha config ID"),
    )
)]
pub async fn get_captcha_config(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<CaptchaConfig>, ApiError> {
    let config = state
        .services
        .content_admin
        .get_captcha_config(id)
        .await?
        .ok_or_else(|| ApiError::not_found("Captcha config not found"))?;
    Ok(Json(config))
}

#[utoipa::path(
    post,
    path = "/captcha-configs/",
    tag = "captcha",
    request_body = CreateCaptchaConfigInput,
    responses(
        (status = 201, description = "Captcha config created successfully", body = CaptchaConfig),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn create_captcha_config(
    State(state): State<AppState>,
    identity: AdminIdentity,
    Json(input): Json<CreateCaptchaConfigInput>,
) -> Result<(StatusCode, Json<CaptchaConfig>), ApiError> {
    let config = state
        .services
        .content_admin
        .create_captcha_config(&identity, input)
        .await?;
    Ok((StatusCode::CREATED, Json(config)))
}

#[utoipa::path(
    patch,
    path = "/captcha-configs/{id}/",
    tag = "captcha",
    request_body = UpdateCaptchaConfigInput,
    responses(
        (status = 200, description = "Captcha config updated successfully", body = CaptchaConfig),
        (status = 404, description = "Captcha config not found"),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Unauthorized"),
    ),
    params(
        ("id" = Uuid, Path, description = "Captcha config ID"),
    )
)]
pub async fn update_captcha_config(
    State(state): State<AppState>,
    identity: AdminIdentity,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateCaptchaConfigInput>,
) -> Result<Json<CaptchaConfig>, ApiError> {
    let config = state
        .services
        .content_admin
        .update_captcha_config(&identity, id, input)
        .await?;
    Ok(Json(config))
}

#[utoipa::path(
    delete,
    path = "/captcha-configs/{id}/",
    tag = "captcha",
    responses(
        (status = 204, description = "Captcha config deleted successfully"),
        (status = 404, description = "Captcha config not found"),
        (status = 401, description = "Unauthorized"),
    ),
    params(
        ("id" = Uuid, Path, description = "Captcha config ID"),
    )
)]
pub async fn delete_captcha_config(
    State(state): State<AppState>,
    identity: AdminIdentity,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    state
        .services
        .content_admin
        .delete_captcha_config(&identity, id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
