use std::{collections::HashMap, fmt::Debug};

use serde::{Deserialize, Serialize};

pub const GRECAPTCHA_ENTERPRISE_URL: &str =
    "https://recaptchaenterprise.googleapis.com/v1/projects/{PROJECT_ID}/assessments";
pub const GRECAPTCHA_ENTERPRISE_DEFAULT_ACTION: &str = "SUBMIT";
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ja3: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ja4: Option<String>,
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
    /// Reason for invalidity when valid=false
    #[serde(default)]
    pub invalid_reason: Option<String>,
    /// Hostname of the page where the token was generated (web keys only)
    #[serde(default)]
    pub hostname: Option<String>,
    /// Action name provided at token generation
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub create_time: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrecaptchaEnterpriseRiskAnalysis {
    pub score: f64,
    #[serde(default)]
    pub reasons: Vec<String>,
    /// Extended verdict reasons (Enterprise tier only)
    #[serde(default)]
    pub extended_verdict_reasons: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TurnstileResponse {
    pub success: bool,
    #[serde(rename = "error-codes")]
    pub error_codes: Vec<String>,
    pub challenge_ts: Option<String>,
    pub hostname: Option<String>,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct TripwireAssessment {
    pub proxy: Option<bool>,
    pub proxy_type: Option<String>,
    pub timestamp: Option<i64>,
    pub source_ip: Option<String>,
    pub key: Option<String>,
    pub uuid: Option<String>,
}

/// Which endpoint(s) a captcha config is used for
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum CaptchaEndpointUsage {
    #[default]
    AuthCode,
    ReAuth,
    All,
}

impl CaptchaEndpointUsage {
    pub fn from_str(s: &str) -> Self {
        match s {
            "re_auth" => Self::ReAuth,
            "all" => Self::All,
            _ => Self::AuthCode,
        }
    }

    pub fn matches_auth_code(&self) -> bool {
        matches!(self, Self::AuthCode | Self::All)
    }

    pub fn matches_reauth(&self) -> bool {
        matches!(self, Self::ReAuth | Self::All)
    }
}

/// Configuration for a captcha provider
#[derive(Clone, Serialize, Deserialize)]
pub struct CaptchaProviderConfig {
    /// Display name for this config (used as key in additional_info)
    pub name: String,
    /// Provider type (e.g., "turnstile", "hcaptcha", "monocle", "custom")
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
    /// Which endpoint this captcha is used for
    #[serde(default)]
    pub endpoint_usage: CaptchaEndpointUsage,
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
            .field("name", &self.name)
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
    /// Not used by reCAPTCHA Enterprise (which derives its URL from project_id).
    #[serde(default)]
    pub url: Option<String>,
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
    /// Score threshold for providers that return a risk score (e.g., reCAPTCHA Enterprise).
    /// Defaults to 0.5 when None.
    #[serde(default)]
    pub score_threshold: Option<f64>,
    /// Google Cloud project ID for reCAPTCHA Enterprise.
    #[serde(default)]
    pub project_id: Option<String>,
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
