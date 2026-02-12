use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Response,
    routing::{delete, get, patch, post},
    Json, Router,
};
use uuid::Uuid;

use crate::{
    auth::AdminSession,
    models::{CaptchaConfig, CreateCaptchaConfigInput, UpdateCaptchaConfigInput},
    repository::captcha_config_repository::CaptchaConfigRepository,
    DefaultAppState,
};

pub fn routes() -> Router<DefaultAppState> {
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
    State(state): State<DefaultAppState>,
) -> Json<Vec<CaptchaConfig>> {
    let configs = state.captcha_config_repo.get_all().await.unwrap();
    Json(configs)
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
    State(state): State<DefaultAppState>,
    Path(id): Path<Uuid>,
) -> Response {
    let config = state.captcha_config_repo.get_by_id(id).await.unwrap();

    match config {
        Some(config) => Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&config).unwrap().into())
            .unwrap(),
        None => Response::builder()
            .status(404)
            .body(axum::body::Body::empty())
            .unwrap(),
    }
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
    State(state): State<DefaultAppState>,
    admin_session: AdminSession,
    Json(input): Json<CreateCaptchaConfigInput>,
) -> Response {
    let updated_by = admin_session.get_admin_email();

    if updated_by.is_none() {
        return Response::builder()
            .status(401)
            .body("Unauthorized: No user information available".into())
            .unwrap();
    }

    match state.captcha_config_repo.create(input, updated_by).await {
        Ok(config) => Response::builder()
            .status(201)
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&config).unwrap().into())
            .unwrap(),
        Err(e) => {
            tracing::error!("Failed to create captcha config: {e:?}");
            Response::builder()
                .status(400)
                .body(format!("Failed to create captcha config: {e}").into())
                .unwrap()
        }
    }
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
    State(state): State<DefaultAppState>,
    admin_session: AdminSession,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateCaptchaConfigInput>,
) -> Response {
    let updated_by = admin_session.get_admin_email();

    if updated_by.is_none() {
        return Response::builder()
            .status(401)
            .body("Unauthorized: No user information available".into())
            .unwrap();
    }

    match state
        .captcha_config_repo
        .update(id, input, updated_by)
        .await
    {
        Ok(config) => Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&config).unwrap().into())
            .unwrap(),
        Err(e) => {
            let status = if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::BAD_REQUEST
            };

            Response::builder()
                .status(status)
                .body(format!("Failed to update captcha config: {e}").into())
                .unwrap()
        }
    }
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
    State(state): State<DefaultAppState>,
    admin_session: AdminSession,
    Path(id): Path<Uuid>,
) -> Response {
    if admin_session.get_admin_email().is_none() {
        return Response::builder()
            .status(401)
            .body("Unauthorized: No user information available".into())
            .unwrap();
    }

    match state.captcha_config_repo.delete(id).await {
        Ok(_) => Response::builder()
            .status(204)
            .body(axum::body::Body::empty())
            .unwrap(),
        Err(e) => {
            tracing::error!("Failed to delete captcha config: {e:?}");
            Response::builder()
                .status(404)
                .body(format!("Failed to delete captcha config: {e}").into())
                .unwrap()
        }
    }
}
