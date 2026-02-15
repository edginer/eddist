use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, patch, post},
    Json, Router,
};
use uuid::Uuid;

use crate::{
    auth::AdminEmail,
    error::ApiError,
    models::{CaptchaConfig, CreateCaptchaConfigInput, UpdateCaptchaConfigInput},
    AppState,
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
    let configs = state.captcha_config_repo.get_all().await?;
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
        .captcha_config_repo
        .get_by_id(id)
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
    AdminEmail(email): AdminEmail,
    Json(input): Json<CreateCaptchaConfigInput>,
) -> Result<(StatusCode, Json<CaptchaConfig>), ApiError> {
    let config = state
        .captcha_config_repo
        .create(input, Some(email))
        .await
        .map_err(|e| ApiError::bad_request(format!("Failed to create captcha config: {e}")))?;
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
    AdminEmail(email): AdminEmail,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateCaptchaConfigInput>,
) -> Result<Json<CaptchaConfig>, ApiError> {
    let config = state
        .captcha_config_repo
        .update(id, input, Some(email))
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                ApiError::not_found("Captcha config not found")
            } else {
                ApiError::bad_request(format!("Failed to update captcha config: {e}"))
            }
        })?;
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
    AdminEmail(_email): AdminEmail,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    state
        .captcha_config_repo
        .delete(id)
        .await
        .map_err(|e| ApiError::not_found(format!("Failed to delete captcha config: {e}")))?;
    Ok(StatusCode::NO_CONTENT)
}
