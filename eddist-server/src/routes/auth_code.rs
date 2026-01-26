use std::collections::HashMap;

use axum::{
    extract::State,
    response::{Html, IntoResponse},
    Form,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use http::HeaderMap;
use serde_json::json;
use time;

use crate::{
    domain::captcha_like::CaptchaLikeConfig,
    error::BbsPostAuthWithCodeError,
    services::{
        auth_with_code_service::{AuthWithCodeServiceInput, AuthWithCodeServiceOutput},
        AppService,
    },
    utils::{get_origin_ip, get_ua},
    AppState,
};

// NOTE: this system will be changed in the future
pub async fn get_auth_code(State(state): State<AppState>) -> impl IntoResponse {
    let mut site_keys =
        state
            .captcha_like_configs
            .iter()
            .filter_map(|config| match config {
                CaptchaLikeConfig::Turnstile { site_key, .. } => Some(("cf_site_key", site_key)),
                CaptchaLikeConfig::Hcaptcha { site_key, .. } => {
                    Some(("hcaptcha_site_key", site_key))
                }
                CaptchaLikeConfig::Monocle { site_key, .. } => Some(("monocle_site_key", site_key)),
                CaptchaLikeConfig::Cap { site_key, .. } => Some(("cap_site_key", site_key)),
                _ => {
                    tracing::warn!(
                        "not implemented yet such captcha like config, ignored: {config:?}",
                    );
                    None
                }
            })
            .collect::<HashMap<_, _>>();

    // Add cap_base_url separately since we need to collect multiple values from Cap config
    let cap_base_url = state
        .captcha_like_configs
        .iter()
        .find_map(|config| match config {
            CaptchaLikeConfig::Cap { base_url, .. } => Some(base_url),
            _ => None,
        });
    if let Some(base_url) = cap_base_url {
        site_keys.insert("cap_base_url", base_url);
    }

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
    jar: CookieJar,
    State(state): State<AppState>,
    Form(form): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    let rate_limit_token = jar
        .get("auth_rate_limit")
        .map(|cookie| cookie.value().to_string());
    let (token, rate_limit_token) = match state
        .services
        .auth_with_code()
        .execute(AuthWithCodeServiceInput {
            code: form["auth-code"].to_string(),
            origin_ip: get_origin_ip(&headers).to_string(),
            user_agent: get_ua(&headers).to_string(),
            captcha_like_configs: state.captcha_like_configs.clone(),
            responses: form,
            rate_limit_token,
        })
        .await
    {
        Ok(AuthWithCodeServiceOutput {
            token,
            rate_limit_token,
        }) => (token, rate_limit_token),
        Err(e) => {
            return if let Some(e) = e.downcast_ref::<BbsPostAuthWithCodeError>() {
                Html(
                    state
                        .template_engine
                        .render("auth-code.post.failed", &json!({ "reason": e.to_string() }))
                        .unwrap(),
                )
                .into_response()
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
                ).into_response()
            };
        }
    };

    let html = state
        .template_engine
        .render("auth-code.post.success", &json!({ "token": token }))
        .unwrap();

    // Set rate limiting cookie if provided by the service
    let updated_jar = if let Some(token_value) = rate_limit_token {
        jar.add(
            Cookie::build(("auth_rate_limit", token_value))
                .max_age(time::Duration::hours(1))
                .http_only(true)
                .same_site(SameSite::Lax)
                .path("/")
                .build(),
        )
    } else {
        jar
    };

    (updated_jar, Html(html)).into_response()
}
