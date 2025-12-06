use axum::{
    extract::State,
    response::Response,
    routing::{get, put},
    Json, Router,
};

use crate::{
    auth::AdminSession,
    models::Terms,
    repository::terms_repository::{TermsRepository, UpdateTermsInput},
    DefaultAppState,
};

pub fn routes() -> Router<DefaultAppState> {
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
pub async fn get_terms(State(state): State<DefaultAppState>) -> Response {
    let terms = state.terms_repo.get_terms().await.unwrap();

    match terms {
        Some(terms) => {
            let admin_terms: Terms = terms.into();
            Response::builder()
                .status(200)
                .body(serde_json::to_string(&admin_terms).unwrap().into())
                .unwrap()
        }
        None => Response::builder()
            .status(404)
            .body("Terms not found".into())
            .unwrap(),
    }
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
    State(state): State<DefaultAppState>,
    admin_session: AdminSession,
    Json(input): Json<UpdateTermsInput>,
) -> Response {
    let updated_by = admin_session.get_admin_email();

    if updated_by.is_none() {
        return Response::builder()
            .status(401)
            .body("Unauthorized: No user information available".into())
            .unwrap();
    }

    match state.terms_repo.update_terms(input, updated_by).await {
        Ok(terms) => {
            let admin_terms: Terms = terms.into();
            Response::builder()
                .status(200)
                .body(serde_json::to_string(&admin_terms).unwrap().into())
                .unwrap()
        }
        Err(e) => {
            tracing::error!("Failed to update terms: {e:?}");
            let status = if e.to_string().contains("not found") {
                404
            } else {
                400
            };

            Response::builder()
                .status(status)
                .body(format!("Failed to update terms: {e}").into())
                .unwrap()
        }
    }
}
