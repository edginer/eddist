use std::collections::HashMap;

use eddist_core::{domain::ip_addr::ReducedIpAddr, utils::is_prod};
use jsonpath_rust::JsonPath;
use serde::{Deserialize, Serialize};

use crate::domain::{
    captcha_like::{
        CaptchaProviderConfig, CaptchaVerificationConfig, HCaptchaResponse, HttpMethod,
        MonocleResponse, PlaceholderResolver, RequestFormat, TurnstileResponse, HCAPTCHA_URL,
        MONOCLE_URL, TURNSTILE_URL,
    },
    utils::SimpleSecret,
};

#[async_trait::async_trait]
pub trait CaptchaClient: Send + Sync {
    async fn verify_captcha(
        &self,
        response: &str,
        ip_addr: &str,
    ) -> Result<CaptchaVerificationOutput, CaptchaVerificationError>;
}

/// Output from captcha verification including captured data
#[derive(Debug, Clone)]
pub struct CaptchaVerificationOutput {
    pub result: CaptchaLikeResult,
    /// Data extracted based on capture_fields config
    pub captured_data: Option<serde_json::Value>,
    /// Provider name for aggregation
    pub provider: String,
}

/// Factory function to create the appropriate captcha client based on provider config
pub fn create_captcha_client(config: &CaptchaProviderConfig) -> Box<dyn CaptchaClient> {
    match config.provider.to_lowercase().as_str() {
        "turnstile" => Box::new(TurnstileClient::new(config.secret.clone())),
        "hcaptcha" => Box::new(HCaptchaClient::new(
            config.secret.clone(),
            config.capture_fields.clone(),
        )),
        "monocle" => Box::new(MonocleClient::new(
            config.secret.clone(),
            config.capture_fields.clone(),
        )),
        // For other providers, use the generic client
        _ => Box::new(GenericCaptchaClient::new(config.clone())),
    }
}

pub struct TurnstileClient {
    client: reqwest::Client,
    secret: SimpleSecret,
}

impl TurnstileClient {
    pub fn new(secret: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            secret: SimpleSecret::new(&secret),
        }
    }
}

#[async_trait::async_trait]
impl CaptchaClient for TurnstileClient {
    async fn verify_captcha(
        &self,
        response: &str,
        ip_addr: &str,
    ) -> Result<CaptchaVerificationOutput, CaptchaVerificationError> {
        let mut form_data = HashMap::new();
        form_data.insert("response", response);
        form_data.insert("remoteip", ip_addr);
        form_data.insert("remoteip_leniency", "relaxed");
        form_data.insert("secret", self.secret.get());

        let res = self
            .client
            .post(TURNSTILE_URL)
            .header("Authorization", self.secret.get())
            .form(&form_data)
            .send()
            .await
            .map_err(CaptchaVerificationError::Request)?;

        let response_text = res
            .text()
            .await
            .map_err(CaptchaVerificationError::Request)?;

        let resp = match serde_json::from_str::<TurnstileResponse>(&response_text) {
            Ok(resp) => resp,
            Err(e) => {
                log::error!(
                    "Failed to parse Turnstile response: {e}, response body: {response_text}"
                );
                return Ok(CaptchaVerificationOutput {
                    result: CaptchaLikeResult::Failure(CaptchaLikeError::FailedToVerifyCaptcha),
                    captured_data: None,
                    provider: "turnstile".to_string(),
                });
            }
        };

        let result = if resp.success {
            CaptchaLikeResult::Success
        } else {
            log::info!("Turnstile response: {resp:?}");
            CaptchaLikeResult::Failure(CaptchaLikeError::FailedToVerifyCaptcha)
        };

        Ok(CaptchaVerificationOutput {
            result,
            captured_data: None,
            provider: "turnstile".to_string(),
        })
    }
}

pub struct HCaptchaClient {
    client: reqwest::Client,
    secret: SimpleSecret,
    capture_fields: Vec<String>,
}

impl HCaptchaClient {
    pub fn new(secret: String, capture_fields: Vec<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            secret: SimpleSecret::new(&secret),
            capture_fields,
        }
    }

    fn extract_captured_data(&self, resp: &HCaptchaResponse) -> Option<serde_json::Value> {
        if self.capture_fields.is_empty() {
            return None;
        }

        // Convert HCaptchaResponse to JSON for field extraction
        let resp_json = serde_json::to_value(resp).ok()?;

        let mut captured = serde_json::Map::new();
        for field in &self.capture_fields {
            if let Some(value) = resp_json.get(field) {
                captured.insert(field.clone(), value.clone());
            }
        }

        if captured.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(captured))
        }
    }
}

#[async_trait::async_trait]
impl CaptchaClient for HCaptchaClient {
    async fn verify_captcha(
        &self,
        response: &str,
        ip_addr: &str,
    ) -> Result<CaptchaVerificationOutput, CaptchaVerificationError> {
        let mut form_data = HashMap::new();
        form_data.insert("response", response);
        form_data.insert("secret", self.secret.get());
        form_data.insert("remoteip", ip_addr);

        let res = self
            .client
            .post(HCAPTCHA_URL)
            .form(&form_data)
            .send()
            .await
            .map_err(CaptchaVerificationError::Request)?;

        let resp = match res.json::<HCaptchaResponse>().await {
            Ok(resp) => resp,
            Err(e) => {
                log::error!("Failed to parse HCaptcha response: {e}");
                return Err(CaptchaVerificationError::Request(e));
            }
        };

        let captured_data = self.extract_captured_data(&resp);

        let result = if resp.success {
            CaptchaLikeResult::Success
        } else {
            log::info!("HCaptcha response: {resp:?}");
            CaptchaLikeResult::Failure(CaptchaLikeError::FailedToVerifyCaptcha)
        };

        Ok(CaptchaVerificationOutput {
            result,
            captured_data,
            provider: "hcaptcha".to_string(),
        })
    }
}

pub struct MonocleClient {
    client: reqwest::Client,
    token: SimpleSecret,
    capture_fields: Vec<String>,
}

impl MonocleClient {
    pub fn new(token: String, capture_fields: Vec<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            token: SimpleSecret::new(&token),
            capture_fields,
        }
    }

    fn extract_captured_data(&self, resp: &MonocleResponse) -> Option<serde_json::Value> {
        if self.capture_fields.is_empty() {
            return None;
        }

        // Convert MonocleResponse to JSON for field extraction
        let resp_json = serde_json::to_value(resp).ok()?;

        let mut captured = serde_json::Map::new();
        for field in &self.capture_fields {
            if let Some(value) = resp_json.get(field) {
                captured.insert(field.clone(), value.clone());
            }
        }

        if captured.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(captured))
        }
    }
}

#[async_trait::async_trait]
impl CaptchaClient for MonocleClient {
    async fn verify_captcha(
        &self,
        response: &str,
        ip_addr: &str,
    ) -> Result<CaptchaVerificationOutput, CaptchaVerificationError> {
        let response = response.to_string();

        let verify_ip = |v4ip: Option<&str>, v6ip: Option<&str>| {
            let from_client_ip_addr = ReducedIpAddr::from(ip_addr.to_string());

            // Check if either IPv4 or IPv6 from spur.us matches the client IP
            let v4_match = v4ip
                .map(|monocle_v4ip| {
                    from_client_ip_addr == ReducedIpAddr::from(monocle_v4ip.to_string())
                })
                .unwrap_or(false);

            let v6_match = v6ip
                .map(|monocle_v6ip| {
                    from_client_ip_addr == ReducedIpAddr::from(monocle_v6ip.to_string())
                })
                .unwrap_or(false);

            // Return true if either IP version matches
            v4_match || v6_match
        };

        let res = self
            .client
            .post(MONOCLE_URL)
            .header("Content-Type", "text/plain; charset=utf-8")
            .header("TOKEN", self.token.get())
            .body(response)
            .send()
            .await
            .map_err(CaptchaVerificationError::Request)?;

        let resp = match res.json::<MonocleResponse>().await {
            Ok(resp) => resp,
            Err(e) => {
                log::error!("Failed to parse Monocle response: {e}");
                return Err(CaptchaVerificationError::Request(e));
            }
        };

        let captured_data = self.extract_captured_data(&resp);

        let result = if matches!(resp.anon, Some(true)) {
            log::info!("Monocle response: {resp:?}");
            CaptchaLikeResult::Failure(CaptchaLikeError::AnonymouseAccess)
        } else if !verify_ip(resp.ip.as_deref(), resp.ipv6.as_deref()) && is_prod() {
            log::info!("Monocle response: {resp:?}");
            CaptchaLikeResult::Failure(CaptchaLikeError::FailedToVerifyIpAddress)
        } else {
            CaptchaLikeResult::Success
        };

        Ok(CaptchaVerificationOutput {
            result,
            captured_data,
            provider: "monocle".to_string(),
        })
    }
}

/// Generic captcha client that handles all providers via configuration
pub struct GenericCaptchaClient {
    client: reqwest::Client,
    config: CaptchaProviderConfig,
}

impl GenericCaptchaClient {
    pub fn new(config: CaptchaProviderConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }

    fn verification(&self) -> &CaptchaVerificationConfig {
        self.config
            .verification
            .as_ref()
            .expect("GenericCaptchaClient requires verification config")
    }

    /// Extract fields from response JSON based on capture_fields config
    fn extract_captured_data(&self, resp: &serde_json::Value) -> Option<serde_json::Value> {
        if self.config.capture_fields.is_empty() {
            return None;
        }

        let mut captured = serde_json::Map::new();
        for field in &self.config.capture_fields {
            if let Ok(results) = resp.query(&format!("$.{field}")) {
                if let Some(value) = results.first() {
                    captured.insert(field.clone(), (*value).clone());
                }
            }
        }

        if captured.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(captured))
        }
    }

    /// Check if response indicates success based on success_path config
    fn check_success(&self, resp: &serde_json::Value) -> bool {
        let cfg = self.verification();
        let path_str = format!("$.{}", cfg.success_path);
        let success = if let Ok(results) = resp.query(&path_str) {
            results.first().and_then(|v| v.as_bool()).unwrap_or(false)
        } else {
            false
        };

        // Apply negation if configured
        if cfg.negate_success {
            !success
        } else {
            success
        }
    }
}

#[async_trait::async_trait]
impl CaptchaClient for GenericCaptchaClient {
    async fn verify_captcha(
        &self,
        response: &str,
        ip_addr: &str,
    ) -> Result<CaptchaVerificationOutput, CaptchaVerificationError> {
        let cfg = self.verification();
        let url = self
            .config
            .resolve_placeholders(&cfg.url, response, ip_addr);

        let mut req = match cfg.method {
            HttpMethod::Post => self.client.post(&url),
            HttpMethod::Get => self.client.get(&url),
        };

        req = req.headers(
            cfg.headers
                .iter()
                .map(|(k, v)| {
                    let resolved_value = self.config.resolve_placeholders(v, response, ip_addr);
                    (
                        k.as_str().parse().unwrap(),
                        resolved_value.as_str().parse().unwrap(),
                    )
                })
                .collect(),
        );

        // Build request body based on format
        req = match cfg.request_format {
            RequestFormat::Form => {
                let mut form = HashMap::new();
                form.insert("response", response.to_string());
                form.insert("secret", self.config.secret.clone());
                if cfg.include_ip {
                    form.insert("remoteip", ip_addr.to_string());
                }
                req.form(&form)
            }
            RequestFormat::Json => {
                let mut json = serde_json::json!({
                    "response": response,
                    "secret": self.config.secret,
                });
                if cfg.include_ip {
                    json["remoteip"] = ip_addr.into();
                }
                req.json(&json)
            }
            RequestFormat::PlainText => {
                let body = cfg
                    .body_template
                    .as_ref()
                    .map(|t| self.config.resolve_placeholders(t, response, ip_addr))
                    .unwrap_or_else(|| response.to_string());

                // For PlainText, set Content-Type if not already in headers
                if !cfg.headers.contains_key("Content-Type") {
                    req = req.header("Content-Type", "text/plain; charset=utf-8");
                }
                req.body(body)
            }
        };

        let res = req
            .send()
            .await
            .map_err(CaptchaVerificationError::Request)?;
        let response_text = res
            .text()
            .await
            .map_err(CaptchaVerificationError::Request)?;

        let resp: serde_json::Value = match serde_json::from_str(&response_text) {
            Ok(v) => v,
            Err(e) => {
                log::error!(
                    "Failed to parse {} response: {e}, body: {response_text}",
                    self.config.provider
                );
                return Ok(CaptchaVerificationOutput {
                    result: CaptchaLikeResult::Failure(CaptchaLikeError::FailedToVerifyCaptcha),
                    captured_data: None,
                    provider: self.config.provider.clone(),
                });
            }
        };

        // Extract captured data before checking success
        let captured_data = self.extract_captured_data(&resp);

        // Check success using configured path
        let success = self.check_success(&resp);

        let result = if success {
            CaptchaLikeResult::Success
        } else {
            log::info!("{} response: {:?}", self.config.provider, resp);
            CaptchaLikeResult::Failure(CaptchaLikeError::FailedToVerifyCaptcha)
        };

        Ok(CaptchaVerificationOutput {
            result,
            captured_data,
            provider: self.config.provider.clone(),
        })
    }
}

#[derive(Debug)]
pub enum CaptchaVerificationError {
    Request(reqwest::Error),
}

impl std::fmt::Display for CaptchaVerificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CaptchaVerificationError::Request(e) => write!(f, "Request error: {e}"),
        }
    }
}

impl std::error::Error for CaptchaVerificationError {}

#[derive(thiserror::Error, Debug, Serialize, Deserialize, Clone)]
pub enum CaptchaLikeError {
    #[error("検証に失敗しました")]
    FailedToVerifyCaptcha,
    #[error("不審な回線からの検証は許可されていません")]
    AnonymouseAccess,
    #[error("IPアドレスの検証に失敗しました")]
    FailedToVerifyIpAddress,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum CaptchaLikeResult {
    Success,
    Failure(CaptchaLikeError),
}
