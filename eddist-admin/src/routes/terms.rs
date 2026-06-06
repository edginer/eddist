use axum::{
    Json, Router,
    extract::State,
    routing::{get, put},
};

use crate::{
    AppState, auth::AdminIdentity, error::ApiError, models::Terms,
    repository::terms_repository::UpdateTermsInput,
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
        .services
        .content_admin
        .get_terms()
        .await?
        .ok_or_else(|| ApiError::not_found("Terms not found"))?;
    Ok(Json(terms))
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
    identity: AdminIdentity,
    Json(input): Json<UpdateTermsInput>,
) -> Result<Json<Terms>, ApiError> {
    let terms = state
        .services
        .content_admin
        .update_terms(&identity, input)
        .await?;
    Ok(Json(terms))
}
