use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

/// Typed errors produced by service and repository layer to convey semantic meaning.
/// Wrapping these in `anyhow::Error` lets the `From<anyhow::Error> for ApiError`
/// impl downcast them automatically, so route handlers can just use `?`.
#[derive(thiserror::Error, Debug, Clone)]
pub enum ServiceError {
    #[error("{0}")]
    NotFound(String),

    #[error("{0}")]
    Forbidden(String),

    #[error("{0}")]
    BadRequest(String),

    #[error("{0}")]
    ConfigError(String),
}

#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error(transparent)]
    Internal(anyhow::Error),
}

impl ApiError {
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::BadRequest(msg.into())
    }

    pub fn unauthorized() -> Self {
        Self::Unauthorized("No user information available".into())
    }

    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self::Forbidden(msg.into())
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        // Walk the full source chain so that ServiceErrors wrapped with .context() are still found.
        // downcast_ref on anyhow::Error alone only matches the outermost error.
        let service_err = e
            .chain()
            .find_map(|cause| cause.downcast_ref::<ServiceError>());
        match service_err {
            Some(ServiceError::NotFound(msg)) => ApiError::NotFound(msg.clone()),
            Some(ServiceError::Forbidden(msg)) => ApiError::Forbidden(msg.clone()),
            Some(ServiceError::BadRequest(msg)) => ApiError::BadRequest(msg.clone()),
            Some(ServiceError::ConfigError(_)) => {
                tracing::error!("Configuration error: {e:?}");
                ApiError::Internal(e)
            }
            None => ApiError::Internal(e),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg.clone()),
            ApiError::Internal(err) => {
                tracing::error!("Internal error: {err:?}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
        };

        (status, Json(json!({ "error": message }))).into_response()
    }
}
