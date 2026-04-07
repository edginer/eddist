use std::collections::HashMap;

use axum::{
    Form,
    extract::State,
    response::{Html, IntoResponse},
};
use http::{HeaderMap, HeaderValue};
use serde_json::json;

use crate::{
    AppState,
    error::BbsPostAuthWithCodeError,
    services::{
        AppService, captcha_config_cache::get_cached_captcha_configs_for_reauth,
        reauth_service::ReAuthServiceInput,
    },
    utils::get_origin_ip,
};

use super::auth_code::build_template_variables;

pub async fn get_re_auth(State(state): State<AppState>) -> impl IntoResponse {
    let captcha_configs = get_cached_captcha_configs_for_reauth().await;
    let template_vars = build_template_variables(&captcha_configs);

    let html = state
        .template_engine
        .render("re-auth.get", &template_vars)
        .unwrap();

    let mut resp = Html(html).into_response();
    resp.headers_mut()
        .insert("Cache-Control", HeaderValue::from_static("private"));
    resp
}

pub async fn post_re_auth(
    headers: HeaderMap,
    State(state): State<AppState>,
    Form(form): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    let captcha_configs = get_cached_captcha_configs_for_reauth().await;
    let temp_key = form.get("temp_key").cloned().unwrap_or_default();
    match state
        .services
        .reauth()
        .execute(ReAuthServiceInput {
            temp_key,
            origin_ip: get_origin_ip(&headers).to_string(),
            captcha_configs,
            responses: form,
        })
        .await
    {
        Ok(()) => Html(
            state
                .template_engine
                .render("re-auth.post.success", &json!({}))
                .unwrap(),
        )
        .into_response(),
        Err(e) => {
            let reason = if let Some(e) = e.downcast_ref::<BbsPostAuthWithCodeError>() {
                e.to_string()
            } else {
                log::error!("Failed to re-auth: {e:?}");
                "不明な理由です（認証に失敗した可能性があります）".to_string()
            };
            Html(
                state
                    .template_engine
                    .render("re-auth.post.failed", &json!({ "reason": reason }))
                    .unwrap(),
            )
            .into_response()
        }
    }
}
