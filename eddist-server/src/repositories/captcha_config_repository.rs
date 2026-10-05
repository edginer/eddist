use std::collections::HashMap;

use anyhow::{Context, anyhow, ensure};
use eddist_core::utils::slugify;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::domain::captcha_like::{
    CaptchaEndpointUsage, CaptchaProviderConfig, CaptchaVerificationConfig, CaptchaWidgetMetadata,
    HttpMethod, RequestFormat,
};

#[derive(Debug, Clone)]
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
    endpoint_usage: String,
}

/// Get default widget config for first-class providers
/// Turnstile uses data-response-field-name to generate unique form field names per config,
/// preventing collisions when multiple configs share the same provider type.
fn get_default_widget_config(
    provider: &str,
    site_key: &str,
    name: &str,
) -> Option<CaptchaWidgetMetadata> {
    let slug = slugify(name);
    match provider {
        "turnstile" => Some(CaptchaWidgetMetadata {
            form_field_name: format!("cf-turnstile-response-{slug}"),
            script_url: "https://challenges.cloudflare.com/turnstile/v0/api.js".to_string(),
            widget_html: format!(
                r#"<div class="cf-turnstile" data-sitekey="{site_key}" data-response-field-name="cf-turnstile-response-{slug}"></div>"#,
            ),
            script_handler: None,
        }),
        "hcaptcha" => Some(CaptchaWidgetMetadata {
            form_field_name: "h-captcha-response".to_string(),
            script_url: "https://js.hcaptcha.com/1/api.js?recaptchacompat=off".to_string(),
            widget_html: format!(
                r#"<div class="h-captcha" data-sitekey="{}"></div>"#,
                site_key
            ),
            script_handler: None,
        }),
        "monocle" => Some(CaptchaWidgetMetadata {
            form_field_name: "monocle".to_string(),
            script_url: format!("https://mcl.spur.us/d/mcl.js?tk={}", site_key),
            widget_html: String::new(),
            script_handler: None,
        }),
        "layer3intel_tripwire" => Some(CaptchaWidgetMetadata {
            form_field_name: "tripwire_token".to_string(),
            script_url: format!(
                "https://cdn.layer3intel.com/tripwire.min.js?key={}",
                site_key
            ),
            widget_html: String::new(),
            // Intercept form submission manually so token injection is not sensitive
            // to when the async script finishes loading relative to DOMContentLoaded.
            // Uses requestSubmit() (not submit()) so other async captcha handlers on the
            // same form still get their submit listener re-invoked instead of being bypassed.
            script_handler: Some(
                r#"(function() {
  var form = document.getElementById('login-form');
  form.addEventListener('submit', function(e) {
    if (document.querySelector('input[name="tripwire_token"]')) return;
    if (typeof window.Tripwire === 'undefined') return;
    e.preventDefault();
    var self = this;
    window.Tripwire.run().then(function(result) {
      var input = document.createElement('input');
      input.type = 'hidden';
      input.name = 'tripwire_token';
      input.value = result.token;
      self.appendChild(input);
      self.requestSubmit();
    }).catch(function() { self.requestSubmit(); });
  });
})();"#
                    .to_string(),
            ),
        }),
        _ => None,
    }
}

/// Verification config as stored in JSON
#[derive(Debug, Clone, serde::Deserialize)]
struct StoredVerificationConfig {
    #[serde(default)]
    pub url: Option<String>,
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
    #[serde(default)]
    pub score_threshold: Option<f64>,
    #[serde(default)]
    pub project_id: Option<String>,
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
            score_threshold: stored.score_threshold,
            project_id: stored.project_id,
        }
    }
}

impl TryFrom<CaptchaConfigRow> for CaptchaProviderConfig {
    type Error = anyhow::Error;

    fn try_from(row: CaptchaConfigRow) -> anyhow::Result<Self> {
        let config_id = row.id;
        convert_captcha_config_row(row)
            .with_context(|| format!("invalid CAPTCHA config {config_id}"))
    }
}

fn convert_captcha_config_row(row: CaptchaConfigRow) -> anyhow::Result<CaptchaProviderConfig> {
    let capture_fields: Vec<String> = row
        .capture_fields
        .map(|v| serde_json::from_value(v).map_err(|_| anyhow!("invalid capture fields JSON")))
        .transpose()?
        .unwrap_or_default();

    let verification: Option<CaptchaVerificationConfig> = row
        .verification
        .map(|v| {
            serde_json::from_value::<StoredVerificationConfig>(v)
                .map_err(|_| anyhow!("invalid verification JSON"))
        })
        .transpose()?
        .map(Into::into);

    ensure!(
        matches!(row.endpoint_usage.as_str(), "auth_code" | "re_auth" | "all"),
        "invalid endpoint usage"
    );

    let default_widget =
        get_default_widget_config(&row.provider.to_lowercase(), &row.site_key, &row.name);
    // Admin stores each empty widget column as NULL independently; a partial widget
    // must not override a provider default.
    let widget = match (
        row.widget_form_field_name,
        row.widget_script_url,
        row.widget_html,
        default_widget,
    ) {
        (Some(form_field_name), Some(script_url), Some(widget_html), _) => CaptchaWidgetMetadata {
            form_field_name,
            script_url,
            widget_html,
            script_handler: row.widget_script_handler,
        },
        (_, _, _, Some(default_widget)) => default_widget,
        (Some(form_field_name), script_url, widget_html, None) => CaptchaWidgetMetadata {
            form_field_name,
            script_url: script_url.unwrap_or_default(),
            widget_html: widget_html.unwrap_or_default(),
            script_handler: row.widget_script_handler,
        },
        (None, _, _, None) => return Err(anyhow!("widget config is required for this provider")),
    };

    let config = CaptchaProviderConfig {
        name: row.name,
        provider: row.provider,
        site_key: row.site_key,
        secret: row.secret,
        base_url: row.base_url,
        widget,
        endpoint_usage: CaptchaEndpointUsage::from_str(&row.endpoint_usage),
        capture_fields,
        verification,
    };
    config.validate()?;
    Ok(config)
}

#[async_trait::async_trait]
pub trait CaptchaConfigRepository: Send + Sync + 'static {
    async fn get_active_captcha_configs(&self) -> anyhow::Result<Vec<CaptchaProviderConfig>>;
}

#[derive(Debug, Clone)]
pub struct CaptchaConfigRepositoryImpl {
    pool: MySqlPool,
}

impl CaptchaConfigRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl CaptchaConfigRepository for CaptchaConfigRepositoryImpl {
    async fn get_active_captcha_configs(&self) -> anyhow::Result<Vec<CaptchaProviderConfig>> {
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
                display_order,
                endpoint_usage
            FROM captcha_configs
            WHERE is_active = 1
            ORDER BY display_order ASC, created_at ASC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        collect_valid_configs(rows)
    }
}

fn collect_valid_configs(
    rows: Vec<CaptchaConfigRow>,
) -> anyhow::Result<Vec<CaptchaProviderConfig>> {
    let active_rows = rows.len();
    let configs: Vec<_> = rows
        .into_iter()
        .filter_map(|row| match CaptchaProviderConfig::try_from(row) {
            Ok(config) => Some(config),
            Err(e) => {
                tracing::error!(error = %format_args!("{e:#}"), "skipping invalid CAPTCHA config");
                None
            }
        })
        .collect();
    // An empty snapshot disables CAPTCHA, so broken rows must not produce one.
    ensure!(
        active_rows == 0 || !configs.is_empty(),
        "all {active_rows} active CAPTCHA configs are invalid"
    );
    Ok(configs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn row(provider: &str) -> CaptchaConfigRow {
        CaptchaConfigRow {
            id: Uuid::nil(),
            name: "test".into(),
            provider: provider.into(),
            site_key: "site-key".into(),
            secret: "secret-value".into(),
            base_url: None,
            widget_form_field_name: None,
            widget_script_url: None,
            widget_html: None,
            widget_script_handler: None,
            capture_fields: None,
            verification: None,
            is_active: true,
            display_order: 0,
            endpoint_usage: "auth_code".into(),
        }
    }

    fn custom_row() -> CaptchaConfigRow {
        let mut row = row("custom");
        row.widget_form_field_name = Some("captcha-response".into());
        row.widget_script_url = Some("https://captcha.example/widget.js".into());
        row.widget_html = Some(String::new());
        row.verification = Some(json!({ "url": "https://captcha.example/verify" }));
        row
    }

    #[test]
    fn first_class_providers_use_valid_defaults() {
        for provider in ["turnstile", "hcaptcha", "monocle", "TURNSTILE"] {
            let config = CaptchaProviderConfig::try_from(row(provider)).unwrap();
            assert!(!config.widget.script_url.is_empty());
        }
    }

    #[test]
    fn invalid_json_is_rejected_without_logging_its_values() {
        let mut row = custom_row();
        row.verification = Some(json!({ "headers": "sensitive-header-value" }));
        let error = CaptchaProviderConfig::try_from(row).unwrap_err();
        let message = format!("{error:#}");
        assert!(message.contains("invalid verification JSON"));
        assert!(message.contains(&Uuid::nil().to_string()));
        assert!(!message.contains("sensitive-header-value"));

        let mut row = custom_row();
        row.capture_fields = Some(json!({ "invalid": "sensitive-data" }));
        assert!(CaptchaProviderConfig::try_from(row).is_err());
    }

    #[test]
    fn generic_provider_requires_verification_and_a_widget_field_name() {
        let mut row = custom_row();
        row.verification = None;
        assert!(CaptchaProviderConfig::try_from(row).is_err());
        let mut row = custom_row();
        row.widget_form_field_name = None;
        assert!(CaptchaProviderConfig::try_from(row).is_err());
    }

    #[test]
    fn partial_widget_columns_are_treated_as_empty() {
        let mut row = custom_row();
        row.widget_html = None;
        let config = CaptchaProviderConfig::try_from(row).unwrap();
        assert_eq!(config.widget.widget_html, "");

        let mut row = custom_row();
        row.widget_script_url = None;
        assert_eq!(
            CaptchaProviderConfig::try_from(row)
                .unwrap()
                .widget
                .script_url,
            ""
        );

        for (field_name, script_url) in [
            (None, Some("https://example.com/x.js")),
            (Some("custom-response"), None),
        ] {
            let mut first_class = self::row("turnstile");
            first_class.widget_form_field_name = field_name.map(Into::into);
            first_class.widget_script_url = script_url.map(Into::into);
            let config = CaptchaProviderConfig::try_from(first_class).unwrap();
            assert!(
                config
                    .widget
                    .script_url
                    .contains("challenges.cloudflare.com")
            );
            assert!(config.widget.widget_html.contains("cf-turnstile"));
        }

        let mut first_class = self::row("turnstile");
        first_class.widget_form_field_name = Some("custom-response".into());
        first_class.widget_script_url = Some("https://example.com/x.js".into());
        first_class.widget_html = Some(String::new());
        let config = CaptchaProviderConfig::try_from(first_class).unwrap();
        assert_eq!(config.widget.form_field_name, "custom-response");
    }

    #[test]
    fn first_class_providers_ignore_unused_fields() {
        let mut row = row("turnstile");
        row.base_url = Some("not a url".into());
        row.verification = Some(json!({
            "url": "file:///x",
            "headers": { "bad header": "value" },
            "success_path": "[",
        }));
        assert!(CaptchaProviderConfig::try_from(row).is_ok());
    }

    #[test]
    fn relative_script_urls_are_accepted() {
        for url in [
            "/static/captcha.js",
            "//cdn.example/captcha.js",
            "js/captcha.js",
        ] {
            let mut row = custom_row();
            row.widget_script_url = Some(url.into());
            assert!(CaptchaProviderConfig::try_from(row).is_ok(), "{url}");
        }
        for url in ["javascript:alert(1)", "/{{missing}}.js"] {
            let mut row = custom_row();
            row.widget_script_url = Some(url.into());
            assert!(CaptchaProviderConfig::try_from(row).is_err(), "{url}");
        }
    }

    #[test]
    fn invalid_rows_are_skipped_but_cannot_empty_the_snapshot() {
        let mut invalid = row("turnstile");
        invalid.secret.clear();
        let configs = collect_valid_configs(vec![invalid.clone(), row("hcaptcha")]).unwrap();
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].provider, "hcaptcha");

        assert!(collect_valid_configs(vec![invalid]).is_err());
        assert!(collect_valid_configs(Vec::new()).unwrap().is_empty());
    }

    #[test]
    fn generic_provider_validates_resolved_urls_headers_and_paths() {
        let mut valid = custom_row();
        valid.base_url = Some("https://captcha.example".into());
        valid.verification = Some(json!({
            "url": "{{base_url}}/verify",
            "headers": { "Authorization": "Bearer {{secret}}" },
        }));
        assert!(CaptchaProviderConfig::try_from(valid.clone()).is_ok());

        for verification in [
            json!({}),
            json!({ "url": "file:///secret-value" }),
            json!({ "url": "{{missing}}/verify" }),
            json!({ "url": "https://captcha.example/verify", "headers": { "bad header": "value" } }),
            json!({ "url": "https://captcha.example/verify", "headers": { "Authorization": "secret-value\ninvalid" } }),
            json!({ "url": "https://captcha.example/verify", "success_path": "[" }),
        ] {
            let mut invalid = valid.clone();
            invalid.verification = Some(verification);
            let error = CaptchaProviderConfig::try_from(invalid).unwrap_err();
            assert!(!format!("{error:#}").contains("secret-value"));
        }
    }

    #[test]
    fn enterprise_requires_project_and_valid_score_threshold() {
        let mut row = custom_row();
        row.provider = "recaptcha_enterprise".into();
        row.verification = Some(json!({ "project_id": "test-project", "score_threshold": 0.5 }));
        assert!(CaptchaProviderConfig::try_from(row.clone()).is_ok());
        for verification in [
            json!({}),
            json!({ "project_id": "" }),
            json!({ "project_id": "invalid/project" }),
            json!({ "project_id": "test-project", "score_threshold": 1.1 }),
            json!({ "project_id": "test-project", "score_threshold": -0.1 }),
        ] {
            row.verification = Some(verification);
            assert!(CaptchaProviderConfig::try_from(row.clone()).is_err());
        }
    }

    #[test]
    fn tripwire_rejects_invalid_private_key() {
        assert!(CaptchaProviderConfig::try_from(row("layer3intel_tripwire")).is_err());
    }

    #[test]
    fn required_fields_and_endpoint_usage_are_validated() {
        let mut row = row("turnstile");
        row.secret.clear();
        assert!(CaptchaProviderConfig::try_from(row).is_err());
        let mut row = custom_row();
        row.endpoint_usage = "typo".into();
        assert!(CaptchaProviderConfig::try_from(row).is_err());
    }
}
