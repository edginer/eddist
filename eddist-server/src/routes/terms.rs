use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;

use crate::{app::AppState, repositories::terms_repository::TermsRepository};

/// Public API response for terms (excludes internal fields like id)
#[derive(Debug, Serialize)]
pub struct TermsResponse {
    pub content: String,
}

impl From<eddist_core::domain::terms::Terms> for TermsResponse {
    fn from(terms: eddist_core::domain::terms::Terms) -> Self {
        TermsResponse {
            content: terms.content,
        }
    }
}

pub async fn get_terms(State(state): State<AppState>) -> impl IntoResponse {
    match state.terms_repo.get_terms().await {
        Ok(Some(terms)) => {
            let response: TermsResponse = terms.into();
            let mut resp = Json(response).into_response();
            resp.headers_mut()
                .insert("Cache-Control", "s-maxage=300".parse().unwrap());
            resp
        }
        Ok(None) => (StatusCode::NOT_FOUND, "Terms not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to get terms: {e:?}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
        }
    }
}
