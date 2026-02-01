use std::{collections::HashMap, fmt::Debug};

use serde::{Deserialize, Serialize};

pub const GRECAPTCHA_ENTERPRISE_URL: &str =
    "https://recaptchaenterprise.googleapis.com/v1/projects/{PROJECT_ID}/assessments";
pub const TURNSTILE_URL: &str = "https://challenges.cloudflare.com/turnstile/v0/siteverify";
pub const HCAPTCHA_URL: &str = "https://api.hcaptcha.com/siteverify";
pub const MONOCLE_URL: &str = "https://decrypt.mcl.spur.us/api/v1/assessment";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrecaptchaEnterpriseRequest {
    pub token: String,
    pub site_key: String,
    pub user_agent: String,
    pub user_ip_address: String,
    pub ja3: Option<String>,
    pub expected_action: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrecaptchaEnterpriseResponse {
    pub token_properties: GrecaptchaEnterpriseTokenProperties,
    pub risk_analysis: GrecaptchaEnterpriseRiskAnalysis,
    pub event: GrecaptchaEnterpriseRequest,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrecaptchaEnterpriseTokenProperties {
    pub valid: bool,
    pub hostname: String,
    pub action: String,
    pub create_time: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GrecaptchaEnterpriseRiskAnalysis {
    pub score: f64,
    pub reasons: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TurnstileResponse {
    pub success: bool,
    #[serde(rename = "error-codes")]
    pub error_codes: Vec<String>,
    pub challenge_ts: String,
    pub hostname: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HCaptchaResponse {
    pub success: bool,
    pub challenge_ts: Option<String>,
    pub hostname: Option<String>,
    pub credit: Option<bool>,
    #[serde(rename = "error-codes")]
    pub error_codes: Option<Vec<String>>,
    pub score: Option<f64>,           // Enterprise feature
    pub score_reason: Option<String>, // Enterprise feature
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MonocleResponse {
    pub vpn: Option<bool>,
    pub proxied: Option<bool>,
    pub anon: Option<bool>,
    pub ip: Option<String>,
    pub ipv6: Option<String>,
    pub ts: Option<String>,
    pub complete: Option<bool>,
    pub id: Option<String>,
    pub sid: Option<String>,
}

/// Configuration for a captcha provider
#[derive(Clone, Serialize, Deserialize)]
pub struct CaptchaProviderConfig {
    /// Provider name (e.g., "turnstile", "hcaptcha", "monocle", "cap")
    pub provider: String,
    /// Site key for the captcha widget
    pub site_key: String,
    /// Secret key for verification API
    pub secret: String,
    /// Base URL for self-hosted providers (e.g., Cap)
    #[serde(default)]
    pub base_url: Option<String>,
    /// Widget configuration for frontend rendering
    pub widget: CaptchaWidgetMetadata,
    /// Fields to capture from the response and store in additional_info
    #[serde(default)]
    pub capture_fields: Vec<String>,
    /// Verification API configuration (only for custom/generic providers)
    #[serde(default)]
    pub verification: Option<CaptchaVerificationConfig>,
}

impl Debug for CaptchaProviderConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CaptchaProviderConfig")
            .field("provider", &self.provider)
            .field("site_key", &self.site_key)
            .field("secret", &"[REDACTED]")
            .field("base_url", &self.base_url)
            .field("widget", &self.widget)
            .field("capture_fields", &self.capture_fields)
            .field("verification", &self.verification)
            .finish()
    }
}

/// Metadata for rendering the captcha widget in the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptchaWidgetMetadata {
    /// Form field name for the captcha response (e.g., "cf-turnstile-response")
    pub form_field_name: String,
    /// Script URL for loading the captcha widget (supports {{site_key}} placeholder)
    pub script_url: String,
    /// HTML for rendering the widget (supports {{site_key}}, {{base_url}} placeholders)
    pub widget_html: String,
    /// Optional JavaScript code for event handling (e.g., Cap widget solve event)
    #[serde(default)]
    pub script_handler: Option<String>,
}

/// Configuration for the captcha verification API (only for custom/generic providers)
#[derive(Clone, Serialize, Deserialize)]
pub struct CaptchaVerificationConfig {
    /// Verification API URL (supports {{base_url}}, {{site_key}} placeholders)
    pub url: String,
    /// HTTP method (defaults to POST)
    #[serde(default)]
    pub method: HttpMethod,
    /// Request body format
    #[serde(default)]
    pub request_format: RequestFormat,
    /// Custom headers (supports {{secret}}, {{site_key}} placeholders)
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Body template for PlainText format (supports {{response}} placeholder)
    #[serde(default)]
    pub body_template: Option<String>,
    /// JSONPath to the success field in the response (default: "success")
    #[serde(default = "default_success_path")]
    pub success_path: String,
    /// Whether to include the client IP address in the request
    #[serde(default)]
    pub include_ip: bool,
    /// Whether the success condition is negated
    #[serde(default)]
    pub negate_success: bool,
}

fn default_success_path() -> String {
    "success".to_string()
}

impl Debug for CaptchaVerificationConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CaptchaVerificationConfig")
            .field("url", &self.url)
            .field("method", &self.method)
            .field("request_format", &self.request_format)
            .field("headers", &"[REDACTED]")
            .field("body_template", &self.body_template)
            .field("success_path", &self.success_path)
            .field("include_ip", &self.include_ip)
            .field("negate_success", &self.negate_success)
            .finish()
    }
}

/// HTTP method for the verification request
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum HttpMethod {
    #[default]
    Post,
    Get,
}

/// Request body format for the verification API
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum RequestFormat {
    /// application/x-www-form-urlencoded
    #[default]
    Form,
    /// application/json
    Json,
    /// text/plain (for Monocle-style raw body)
    PlainText,
}

/// Helper trait to resolve placeholders in configuration strings
pub trait PlaceholderResolver {
    fn resolve_placeholders(&self, template: &str, response: &str, ip_addr: &str) -> String;
}

impl PlaceholderResolver for CaptchaProviderConfig {
    fn resolve_placeholders(&self, template: &str, response: &str, ip_addr: &str) -> String {
        template
            .replace("{{base_url}}", self.base_url.as_deref().unwrap_or(""))
            .replace("{{site_key}}", &self.site_key)
            .replace("{{secret}}", &self.secret)
            .replace("{{response}}", response)
            .replace("{{ip}}", ip_addr)
    }
}
