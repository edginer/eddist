use axum::{
    extract::State,
    routing::{get, put},
    Json, Router,
};

use crate::{
    auth::AdminEmail, error::ApiError, models::Terms,
    repository::terms_repository::UpdateTermsInput, AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/terms", get(get_terms))
        .route("/terms", put(update_terms))
}

#[utoipa::path(
    get,
    path = "/terms/",
    responses(
        (status = 200, description = "Get terms successfully", body = Terms),
        (status = 404, description = "Terms not found"),
    )
)]
pub async fn get_terms(State(state): State<AppState>) -> Result<Json<Terms>, ApiError> {
    let terms = state
        .terms_repo
        .get_terms()
        .await?
        .ok_or_else(|| ApiError::not_found("Terms not found"))?;
    Ok(Json(terms.into()))
}

#[utoipa::path(
    put,
    path = "/terms/",
    request_body = UpdateTermsInput,
    responses(
        (status = 200, description = "Terms updated successfully", body = Terms),
        (status = 404, description = "Terms not found"),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Unauthorized"),
    )
)]
pub async fn update_terms(
    State(state): State<AppState>,
    AdminEmail(email): AdminEmail,
    Json(input): Json<UpdateTermsInput>,
) -> Result<Json<Terms>, ApiError> {
    let terms = state
        .terms_repo
        .update_terms(input, Some(email))
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                ApiError::not_found("Terms not found")
            } else {
                ApiError::bad_request(format!("Failed to update terms: {e}"))
            }
        })?;
    Ok(Json(terms.into()))
}
