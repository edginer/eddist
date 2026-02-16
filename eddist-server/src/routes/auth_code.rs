use std::collections::HashMap;

use axum::{
    extract::State,
    response::{Html, IntoResponse},
    Form,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use http::HeaderMap;
use serde::Serialize;
use serde_json::json;
use time;

use crate::{
    domain::captcha_like::CaptchaProviderConfig,
    error::BbsPostAuthWithCodeError,
    services::{
        server_settings_cache::{get_server_setting_bool, ServerSettingKey},
        auth_with_code_service::{AuthWithCodeServiceInput, AuthWithCodeServiceOutput},
        bind_token_to_user_service::BindTokenToUserServiceInput,
        captcha_config_cache::get_cached_captcha_configs,
        AppService,
    },
    utils::{get_origin_ip, get_ua},
    AppState,
};

/// Script to be loaded for a captcha widget
#[derive(Debug, Serialize)]
struct CaptchaScript {
    url: String,
}

/// Widget HTML to be rendered
#[derive(Debug, Serialize)]
struct CaptchaWidget {
    html: String,
}

/// JavaScript event handler code
#[derive(Debug, Serialize)]
struct CaptchaHandler {
    code: String,
}

/// Resolve placeholders in a template string
fn resolve_placeholders(template: &str, site_key: &str, base_url: Option<&str>) -> String {
    template
        .replace("{{site_key}}", site_key)
        .replace("{{base_url}}", base_url.unwrap_or(""))
}

/// Build template variables from captcha provider configs
fn build_template_variables(configs: &[CaptchaProviderConfig]) -> serde_json::Value {
    let mut scripts = Vec::<CaptchaScript>::new();
    let mut widgets = Vec::<CaptchaWidget>::new();
    let mut handlers = Vec::<CaptchaHandler>::new();

    for config in configs {
        // Resolve script URL
        let script_url = resolve_placeholders(
            &config.widget.script_url,
            &config.site_key,
            config.base_url.as_deref(),
        );
        scripts.push(CaptchaScript { url: script_url });

        // Resolve widget HTML
        let widget_html = resolve_placeholders(
            &config.widget.widget_html,
            &config.site_key,
            config.base_url.as_deref(),
        );
        if !widget_html.is_empty() {
            widgets.push(CaptchaWidget { html: widget_html });
        }

        // Add script handler if present
        if let Some(handler) = &config.widget.script_handler {
            let resolved_handler =
                resolve_placeholders(handler, &config.site_key, config.base_url.as_deref());
            handlers.push(CaptchaHandler {
                code: resolved_handler,
            });
        }
    }

    json!({
        "captcha_scripts": scripts,
        "captcha_widgets": widgets,
        "captcha_handlers": handlers,
    })
}

// NOTE: this system will be changed in the future
pub async fn get_auth_code(State(state): State<AppState>) -> impl IntoResponse {
    let captcha_configs = get_cached_captcha_configs().await;
    let template_vars = build_template_variables(&captcha_configs);

    let html = state
        .template_engine
        .render("auth-code.get", &template_vars)
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
    let captcha_configs = get_cached_captcha_configs().await;
    let (token, authed_token_id, rate_limit_token) = match state
        .services
        .auth_with_code()
        .execute(AuthWithCodeServiceInput {
            code: form["auth-code"].to_string(),
            origin_ip: get_origin_ip(&headers).to_string(),
            user_agent: get_ua(&headers).to_string(),
            captcha_like_configs: captcha_configs,
            responses: form,
            rate_limit_token,
        })
        .await
    {
        Ok(AuthWithCodeServiceOutput {
            token,
            authed_token_id,
            rate_limit_token,
        }) => (token, authed_token_id, rate_limit_token),
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

    // Auto-bind token to user if logged in
    if let Some(user_sid) = jar.get("user-sid").map(|c| c.value().to_string()) {
        if let Err(e) = state
            .services
            .bind_token_to_user()
            .execute(BindTokenToUserServiceInput {
                user_sid,
                authed_token_id,
            })
            .await
        {
            log::warn!("Failed to auto-bind token to user: {e}");
        }
    }

    let html = state
        .template_engine
        .render(
            "auth-code.post.success",
            &json!({
                "token": token,
                "enable_idp_linking": get_server_setting_bool(ServerSettingKey::EnableIdpLinking).await
            }),
        )
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
