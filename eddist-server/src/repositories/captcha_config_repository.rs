use std::collections::HashMap;

use sqlx::MySqlPool;
use uuid::Uuid;

use crate::domain::captcha_like::{
    CaptchaProviderConfig, CaptchaVerificationConfig, CaptchaWidgetMetadata, HttpMethod,
    RequestFormat,
};

#[derive(Debug, Clone, sqlx::FromRow)]
struct CaptchaConfigRow {
    id: Uuid,
    name: String,
    provider: String,
    site_key: String,
    secret: String,
    base_url: Option<String>,
    widget_form_field_name: Option<String>,
    widget_script_url: Option<String>,
    widget_html: Option<String>,
    widget_script_handler: Option<String>,
    capture_fields: Option<serde_json::Value>,
    verification: Option<serde_json::Value>,
    is_active: bool,
    display_order: i32,
}

/// Get default widget config for first-class providers
fn get_default_widget_config(provider: &str, site_key: &str) -> Option<CaptchaWidgetMetadata> {
    match provider {
        "turnstile" => Some(CaptchaWidgetMetadata {
            form_field_name: "cf-turnstile-response".to_string(),
            script_url: "https://challenges.cloudflare.com/turnstile/v0/api.js".to_string(),
            widget_html: format!(
                r#"<div class="cf-turnstile" data-sitekey="{}"></div>"#,
                site_key
            ),
            script_handler: None,
        }),
        "hcaptcha" => Some(CaptchaWidgetMetadata {
            form_field_name: "h-captcha-response".to_string(),
            script_url: "https://js.hcaptcha.com/1/api.js".to_string(),
            widget_html: format!(r#"<div class="h-captcha" data-sitekey="{}"></div>"#, site_key),
            script_handler: None,
        }),
        "monocle" => Some(CaptchaWidgetMetadata {
            form_field_name: "monocle".to_string(),
            script_url: format!("https://mcl.spur.us/d/mcl.js?tk={}", site_key),
            widget_html: String::new(),
            script_handler: None,
        }),
        _ => None,
    }
}

/// Verification config as stored in JSON
#[derive(Debug, Clone, serde::Deserialize)]
struct StoredVerificationConfig {
    pub url: String,
    #[serde(default)]
    pub method: StoredHttpMethod,
    #[serde(default)]
    pub request_format: StoredRequestFormat,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub body_template: Option<String>,
    #[serde(default = "default_success_path")]
    pub success_path: String,
    #[serde(default)]
    pub include_ip: bool,
    #[serde(default)]
    pub negate_success: bool,
}

fn default_success_path() -> String {
    "success".to_string()
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
enum StoredHttpMethod {
    #[default]
    Post,
    Get,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
enum StoredRequestFormat {
    #[default]
    Form,
    Json,
    PlainText,
}

impl From<StoredHttpMethod> for HttpMethod {
    fn from(method: StoredHttpMethod) -> Self {
        match method {
            StoredHttpMethod::Post => HttpMethod::Post,
            StoredHttpMethod::Get => HttpMethod::Get,
        }
    }
}

impl From<StoredRequestFormat> for RequestFormat {
    fn from(format: StoredRequestFormat) -> Self {
        match format {
            StoredRequestFormat::Form => RequestFormat::Form,
            StoredRequestFormat::Json => RequestFormat::Json,
            StoredRequestFormat::PlainText => RequestFormat::PlainText,
        }
    }
}

impl From<StoredVerificationConfig> for CaptchaVerificationConfig {
    fn from(stored: StoredVerificationConfig) -> Self {
        CaptchaVerificationConfig {
            url: stored.url,
            method: stored.method.into(),
            request_format: stored.request_format.into(),
            headers: stored.headers,
            body_template: stored.body_template,
            success_path: stored.success_path,
            include_ip: stored.include_ip,
            negate_success: stored.negate_success,
        }
    }
}

impl From<CaptchaConfigRow> for CaptchaProviderConfig {
    fn from(row: CaptchaConfigRow) -> Self {
        let capture_fields: Vec<String> = row
            .capture_fields
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        let verification: Option<CaptchaVerificationConfig> = row
            .verification
            .and_then(|v| serde_json::from_value::<StoredVerificationConfig>(v).ok())
            .map(Into::into);

        // Use custom widget config if provided, otherwise use defaults for first-class providers
        let widget = match (
            row.widget_form_field_name,
            row.widget_script_url,
            row.widget_html,
        ) {
            (Some(form_field_name), Some(script_url), Some(widget_html)) => CaptchaWidgetMetadata {
                form_field_name,
                script_url,
                widget_html,
                script_handler: row.widget_script_handler,
            },
            _ => get_default_widget_config(&row.provider, &row.site_key)
                .unwrap_or_else(|| CaptchaWidgetMetadata {
                    form_field_name: "captcha-response".to_string(),
                    script_url: String::new(),
                    widget_html: String::new(),
                    script_handler: None,
                }),
        };

        CaptchaProviderConfig {
            provider: row.provider,
            site_key: row.site_key,
            secret: row.secret,
            base_url: row.base_url,
            widget,
            capture_fields,
            verification,
        }
    }
}

/// Load all active captcha configs from the database
pub async fn get_active_captcha_configs(
    pool: &MySqlPool,
) -> anyhow::Result<Vec<CaptchaProviderConfig>> {
    let rows = sqlx::query_as!(
        CaptchaConfigRow,
        r#"
        SELECT
            id AS "id: Uuid",
            name,
            provider,
            site_key,
            secret,
            base_url,
            widget_form_field_name,
            widget_script_url,
            widget_html,
            widget_script_handler,
            capture_fields AS "capture_fields: serde_json::Value",
            verification AS "verification: serde_json::Value",
            is_active AS "is_active: bool",
            display_order
        FROM captcha_configs
        WHERE is_active = 1
        ORDER BY display_order ASC, created_at ASC
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(CaptchaProviderConfig::from).collect())
}
