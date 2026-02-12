use std::collections::HashMap;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Captcha configuration for API responses
#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct CaptchaConfig {
    pub id: Uuid,
    pub name: String,
    pub provider: String,
    pub site_key: String,
    #[serde(skip_serializing)]
    pub secret: String,
    pub base_url: Option<String>,
    /// Widget config - optional for first-class providers (turnstile, hcaptcha, monocle)
    pub widget: Option<CaptchaWidgetConfig>,
    pub capture_fields: Vec<String>,
    pub verification: Option<CaptchaVerificationConfig>,
    pub is_active: bool,
    pub display_order: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub updated_by: Option<String>,
}

/// Widget configuration for frontend rendering
#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct CaptchaWidgetConfig {
    pub form_field_name: String,
    pub script_url: String,
    pub widget_html: String,
    pub script_handler: Option<String>,
}

/// Verification API configuration for custom providers
#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct CaptchaVerificationConfig {
    pub url: String,
    #[serde(default)]
    pub method: HttpMethod,
    #[serde(default)]
    pub request_format: RequestFormat,
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

/// HTTP method for verification requests
#[derive(Debug, Clone, Default, ToSchema, Serialize, Deserialize, PartialEq)]
pub enum HttpMethod {
    #[default]
    Post,
    Get,
}

/// Request body format for verification API
#[derive(Debug, Clone, Default, ToSchema, Serialize, Deserialize, PartialEq)]
pub enum RequestFormat {
    #[default]
    Form,
    Json,
    PlainText,
}

/// Input for creating a new captcha config
#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct CreateCaptchaConfigInput {
    pub name: String,
    pub provider: String,
    pub site_key: String,
    pub secret: String,
    pub base_url: Option<String>,
    /// Widget config - only required for custom providers
    pub widget: Option<CaptchaWidgetConfig>,
    #[serde(default)]
    pub capture_fields: Vec<String>,
    pub verification: Option<CaptchaVerificationConfig>,
    #[serde(default = "default_is_active")]
    pub is_active: bool,
    #[serde(default)]
    pub display_order: i32,
}

fn default_is_active() -> bool {
    true
}

/// Input for updating an existing captcha config
#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct UpdateCaptchaConfigInput {
    pub name: Option<String>,
    pub provider: Option<String>,
    pub site_key: Option<String>,
    pub secret: Option<String>,
    pub base_url: Option<String>,
    pub widget: Option<CaptchaWidgetConfig>,
    pub capture_fields: Option<Vec<String>>,
    pub verification: Option<CaptchaVerificationConfig>,
    pub is_active: Option<bool>,
    pub display_order: Option<i32>,
}
