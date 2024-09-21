use std::fmt::Debug;

use serde::{Deserialize, Serialize};

pub const GRECAPTCHA_URL: &str = "https://www.google.com/recaptcha/api/siteverify";
pub const GRECAPTCHA_ENTERPRISE_URL: &str =
    "https://recaptchaenterprise.googleapis.com/v1/projects/{PROJECT_ID}/assessments";
pub const TURNSTILE_URL: &str = "https://challenges.cloudflare.com/turnstile/v0/siteverify";
pub const HCAPTCHA_URL: &str = "https://api.hcaptcha.com/siteverify";
pub const MONOCLE_URL: &str = "https://decrypt.mcl.spur.us/api/v1/assessment";

#[derive(Debug, Serialize, Deserialize)]
pub struct GrecaptchaV2Response {
    pub success: bool,
    #[serde(rename = "error-codes")]
    pub error_codes: Option<Vec<String>>,
    pub challenge_ts: String,
    pub hostname: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GrecaptchaV3Response {
    pub success: bool,
    pub score: f64,
    pub action: String,
    #[serde(rename = "error-codes")]
    pub error_codes: Option<Vec<String>>,
    pub challenge_ts: String,
    pub hostname: String,
}

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
    pub challenge_ts: String,
    pub hostname: String,
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

#[derive(Clone, Serialize, Deserialize)]
pub enum CaptchaLikeConfig {
    GrecaptchaV2 { site_key: String, secret: String },
    GrecaptchaV3 { site_key: String, secret: String },
    GrecaptchaEnterprise { site_key: String, secret: String },
    Turnstile { site_key: String, secret: String },
    Hcaptcha { site_key: String, secret: String },
    Monocle { site_key: String, token: String },
}

impl Debug for CaptchaLikeConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaptchaLikeConfig::GrecaptchaV2 { site_key, .. } => f
                .debug_struct("GrecaptchaV2")
                .field("site_key", site_key)
                .field("secret", &"[REDACTED]")
                .finish(),
            CaptchaLikeConfig::GrecaptchaV3 { site_key, .. } => f
                .debug_struct("GrecaptchaV3")
                .field("site_key", site_key)
                .field("secret", &"[REDACTED]")
                .finish(),
            CaptchaLikeConfig::GrecaptchaEnterprise { site_key, .. } => f
                .debug_struct("GrecaptchaEnterprise")
                .field("site_key", site_key)
                .field("secret", &"[REDACTED]")
                .finish(),
            CaptchaLikeConfig::Turnstile { site_key, .. } => f
                .debug_struct("Turnstile")
                .field("site_key", site_key)
                .field("secret", &"[REDACTED]")
                .finish(),
            CaptchaLikeConfig::Hcaptcha { site_key, .. } => f
                .debug_struct("Hcaptcha")
                .field("site_key", site_key)
                .field("secret", &"[REDACTED]")
                .finish(),
            CaptchaLikeConfig::Monocle { site_key, .. } => f
                .debug_struct("Monocle")
                .field("site_key", site_key)
                .field("token", &"[REDACTED]")
                .finish(),
        }
    }
}
