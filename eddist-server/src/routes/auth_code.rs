use std::collections::HashMap;

use axum::{
    Form,
    extract::State,
    response::{Html, IntoResponse},
};
use http::HeaderMap;
use serde_json::json;

use crate::{
    AppState,
    domain::captcha_like::CaptchaLikeConfig,
    error::BbsPostAuthWithCodeError,
    services::{
        AppService,
        auth_with_code_service::{AuthWithCodeServiceInput, AuthWithCodeServiceOutput},
    },
    utils::{get_asn_num, get_origin_ip, get_ua},
};

// NOTE: this system will be changed in the future
pub async fn get_auth_code(State(state): State<AppState>) -> impl IntoResponse {
    let site_keys =
        state
            .captcha_like_configs
            .iter()
            .filter_map(|config| match config {
                CaptchaLikeConfig::Turnstile { site_key, .. } => Some(("cf_site_key", site_key)),
                CaptchaLikeConfig::Hcaptcha { site_key, .. } => {
                    Some(("hcaptcha_site_key", site_key))
                }
                CaptchaLikeConfig::Monocle { site_key, .. } => Some(("monocle_site_key", site_key)),
                _ => {
                    tracing::warn!(
                        "not implemented yet such captcha like config, ignored: {config:?}",
                    );
                    None
                }
            })
            .collect::<HashMap<_, _>>();

    let html = state
        .template_engine
        .render("auth-code.get", &serde_json::json!(site_keys))
        .unwrap();

    let mut resp = Html(html).into_response();
    let headers = resp.headers_mut();
    headers.insert("Cache-Control", "private".parse().unwrap());

    resp
}

pub async fn post_auth_code(
    headers: HeaderMap,
    State(state): State<AppState>,
    Form(form): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    let token = match state
        .services
        .auth_with_code()
        .execute(AuthWithCodeServiceInput {
            code: form["auth-code"].to_string(),
            origin_ip: get_origin_ip(&headers).to_string(),
            user_agent: get_ua(&headers).to_string(),
            asn_num: get_asn_num(&headers),
            captcha_like_configs: state.captcha_like_configs.clone(),
            responses: form,
        })
        .await
    {
        Ok(AuthWithCodeServiceOutput { token }) => token,
        Err(e) => {
            return if let Some(e) = e.downcast_ref::<BbsPostAuthWithCodeError>() {
                Html(
                    state
                        .template_engine
                        .render("auth-code.post.failed", &json!({ "reason": e.to_string() }))
                        .unwrap(),
                )
            } else {
                log::error!("Failed to issue authed token: {e:?}");

                Html(
                    state
                        .template_engine
                        .render(
                            "auth-code.post.failed",
                            &json!({ "reason": "不明な理由です（認証に失敗した可能性があります）" }),
                        )
                        .unwrap(),
                )
            };
        }
    };

    let html = state
        .template_engine
        .render("auth-code.post.success", &json!({ "token": token }))
        .unwrap();

    Html(html)
}
