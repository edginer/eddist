use axum::{
    Json,
    extract::Query,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::{
    domain::captcha_like::{CaptchaProviderConfig, CaptchaWidgetMetadata},
    services::captcha_config_cache::{
        get_cached_captcha_configs_for_response_creation,
        get_cached_captcha_configs_for_thread_creation,
    },
};

#[derive(Debug, Deserialize)]
pub struct CaptchaUsageQuery {
    pub usage: String,
}

/// Public, secret-free view of a captcha provider config — only what the
/// frontend needs to render a widget and submit its token.
#[derive(Debug, Serialize)]
pub struct CaptchaConfigPublic {
    pub provider: String,
    pub site_key: String,
    pub base_url: Option<String>,
    pub widget: CaptchaWidgetMetadata,
}

impl From<&CaptchaProviderConfig> for CaptchaConfigPublic {
    fn from(config: &CaptchaProviderConfig) -> Self {
        Self {
            provider: config.provider.clone(),
            site_key: config.site_key.clone(),
            base_url: config.base_url.clone(),
            widget: config.widget.clone(),
        }
    }
}

/// Returns the active captcha widget configs (without secrets) for a posting
/// flow, so the client knows which widgets to render before submitting.
///
/// `usage` must be `thread_creation` or `res_creation` — the same wire
/// vocabulary as `CaptchaEndpointUsage`. Other usages (auth_code/re_auth/...)
/// have their own server-rendered or cross-cutting flows and are intentionally
/// not exposed here.
pub async fn get_api_captcha_configs(Query(params): Query<CaptchaUsageQuery>) -> Response {
    let configs = match params.usage.as_str() {
        "thread_creation" => get_cached_captcha_configs_for_thread_creation().await,
        "res_creation" => get_cached_captcha_configs_for_response_creation().await,
        _ => {
            return (StatusCode::BAD_REQUEST, "invalid usage").into_response();
        }
    };

    let configs: Vec<CaptchaConfigPublic> = configs.iter().map(CaptchaConfigPublic::from).collect();
    Json(configs).into_response()
}
