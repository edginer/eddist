use std::collections::HashMap;

use aes_gcm::{
    Aes256Gcm, KeyInit,
    aead::{Aead, Payload},
};
use base64::Engine;
use eddist_core::{domain::ip_addr::ReducedIpAddr, utils::is_prod};
use jsonpath_rust::JsonPath;
use p256::{
    PublicKey as P256PublicKey, SecretKey as P256SecretKey, ecdh::diffie_hellman,
    pkcs8::DecodePrivateKey,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::domain::{
    captcha_like::{
        CaptchaProviderConfig, CaptchaVerificationConfig, GRECAPTCHA_ENTERPRISE_DEFAULT_ACTION,
        GRECAPTCHA_ENTERPRISE_URL, GrecaptchaEnterpriseRequest, GrecaptchaEnterpriseResponse,
        HCAPTCHA_URL, HCaptchaResponse, HttpMethod, MONOCLE_URL, MonocleResponse,
        PlaceholderResolver, RequestFormat, TURNSTILE_URL, TripwireAssessment, TurnstileResponse,
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

fn extract_fields(source: &impl Serialize, fields: &[String]) -> Option<serde_json::Value> {
    if fields.is_empty() {
        return None;
    }
    let json = serde_json::to_value(source).ok()?;
    let mut captured = serde_json::Map::new();
    for field in fields {
        if let Some(value) = json.get(field) {
            captured.insert(field.clone(), value.clone());
        }
    }
    if captured.is_empty() {
        None
    } else {
        Some(serde_json::Value::Object(captured))
    }
}

/// Factory function to create the appropriate captcha client based on provider config
pub fn create_captcha_client(config: &CaptchaProviderConfig) -> Box<dyn CaptchaClient> {
    let name = config.name.clone();
    match config.provider.to_lowercase().as_str() {
        "turnstile" => Box::new(TurnstileClient::new(name, config.secret.clone())),
        "hcaptcha" => Box::new(HCaptchaClient::new(
            name,
            config.secret.clone(),
            config.capture_fields.clone(),
        )),
        "monocle" => Box::new(MonocleClient::new(
            name,
            config.secret.clone(),
            config.capture_fields.clone(),
        )),
        "recaptcha_enterprise" => Box::new(RecaptchaEnterpriseClient::new(
            name,
            config.secret.clone(),
            config.site_key.clone(),
            config
                .verification
                .as_ref()
                .and_then(|v| v.project_id.clone())
                .unwrap_or_default(),
            config
                .verification
                .as_ref()
                .and_then(|v| v.score_threshold)
                .unwrap_or(0.5),
            config.capture_fields.clone(),
        )),
        "layer3intel_tripwire" => Box::new(Layer3IntelTripwireClient::new(
            name,
            config.secret.clone(),
            config.capture_fields.clone(),
        )),
        // For other providers, use the generic client
        _ => Box::new(GenericCaptchaClient::new(config.clone())),
    }
}

pub struct TurnstileClient {
    name: String,
    client: reqwest::Client,
    secret: SimpleSecret,
}

impl TurnstileClient {
    pub fn new(name: String, secret: String) -> Self {
        Self {
            name,
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
                    provider: self.name.clone(),
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
            provider: self.name.clone(),
        })
    }
}

pub struct HCaptchaClient {
    name: String,
    client: reqwest::Client,
    secret: SimpleSecret,
    capture_fields: Vec<String>,
}

impl HCaptchaClient {
    pub fn new(name: String, secret: String, capture_fields: Vec<String>) -> Self {
        Self {
            name,
            client: reqwest::Client::new(),
            secret: SimpleSecret::new(&secret),
            capture_fields,
        }
    }

    fn extract_captured_data(&self, resp: &HCaptchaResponse) -> Option<serde_json::Value> {
        extract_fields(resp, &self.capture_fields)
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
            provider: self.name.clone(),
        })
    }
}

pub struct MonocleClient {
    name: String,
    client: reqwest::Client,
    token: SimpleSecret,
    capture_fields: Vec<String>,
}

impl MonocleClient {
    pub fn new(name: String, token: String, capture_fields: Vec<String>) -> Self {
        Self {
            name,
            client: reqwest::Client::new(),
            token: SimpleSecret::new(&token),
            capture_fields,
        }
    }

    fn extract_captured_data(&self, resp: &MonocleResponse) -> Option<serde_json::Value> {
        extract_fields(resp, &self.capture_fields)
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

        let response_text = res
            .text()
            .await
            .map_err(CaptchaVerificationError::Request)?;

        let resp = match serde_json::from_str::<MonocleResponse>(&response_text) {
            Ok(resp) => resp,
            Err(e) => {
                log::error!(
                    "Failed to parse Monocle response: {e}, response body: {response_text}"
                );
                return Ok(CaptchaVerificationOutput {
                    result: CaptchaLikeResult::Failure(CaptchaLikeError::FailedToVerifyCaptcha),
                    captured_data: None,
                    provider: self.name.clone(),
                });
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
            provider: self.name.clone(),
        })
    }
}

pub struct Layer3IntelTripwireClient {
    name: String,
    private_key_pem: SimpleSecret,
    capture_fields: Vec<String>,
}

impl Layer3IntelTripwireClient {
    pub fn new(name: String, private_key_pem: String, capture_fields: Vec<String>) -> Self {
        Self {
            name,
            private_key_pem: SimpleSecret::new(&private_key_pem),
            capture_fields,
        }
    }

    fn extract_captured_data(&self, assessment: &TripwireAssessment) -> Option<serde_json::Value> {
        extract_fields(assessment, &self.capture_fields)
    }

    /// Decrypt a JWE compact serialization (ECDH-ES + A256GCM) using the configured private key.
    fn decrypt_jwe(&self, jwe: &str) -> Result<Vec<u8>, CaptchaLikeError> {
        let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;

        // 1. Split into 5 base64url parts
        let parts = jwe.splitn(6, '.').collect::<Vec<&str>>();
        if parts.len() != 5 {
            log::info!("Tripwire JWE: expected 5 parts, got {}", parts.len());
            return Err(CaptchaLikeError::FailedToVerifyCaptcha);
        }
        let (b64_header, _b64_enc_key, b64_iv, b64_ciphertext, b64_tag) =
            (parts[0], parts[1], parts[2], parts[3], parts[4]);

        // 2. Decode header JSON → extract epk x/y coords
        let header_bytes = engine
            .decode(b64_header)
            .map_err(|_| CaptchaLikeError::FailedToVerifyCaptcha)?;
        let header: serde_json::Value = serde_json::from_slice(&header_bytes)
            .map_err(|_| CaptchaLikeError::FailedToVerifyCaptcha)?;
        let epk = header
            .get("epk")
            .ok_or(CaptchaLikeError::FailedToVerifyCaptcha)?;
        let x_b64 = epk
            .get("x")
            .and_then(|v| v.as_str())
            .ok_or(CaptchaLikeError::FailedToVerifyCaptcha)?;
        let y_b64 = epk
            .get("y")
            .and_then(|v| v.as_str())
            .ok_or(CaptchaLikeError::FailedToVerifyCaptcha)?;

        // 3. Parse server private key from PKCS8 PEM
        // Normalize literal \n sequences that appear when the key is stored via a single-line
        // form input or JSON string (e.g. "-----BEGIN PRIVATE KEY-----\nMIG..." with backslash-n).
        let pem = self.private_key_pem.get().replace("\\n", "\n");
        let server_sk = P256SecretKey::from_pkcs8_pem(&pem).map_err(|e| {
            log::error!("Tripwire: failed to parse private key: {e}");
            CaptchaLikeError::FailedToVerifyCaptcha
        })?;

        // 4. Reconstruct ephemeral public key: 0x04 || x (32 bytes) || y (32 bytes)
        let x_bytes = engine
            .decode(x_b64)
            .map_err(|_| CaptchaLikeError::FailedToVerifyCaptcha)?;
        let y_bytes = engine
            .decode(y_b64)
            .map_err(|_| CaptchaLikeError::FailedToVerifyCaptcha)?;
        if x_bytes.len() != 32 || y_bytes.len() != 32 {
            return Err(CaptchaLikeError::FailedToVerifyCaptcha);
        }
        let mut uncompressed = Vec::with_capacity(65);
        uncompressed.push(0x04u8);
        uncompressed.extend_from_slice(&x_bytes);
        uncompressed.extend_from_slice(&y_bytes);
        let epk_pk = P256PublicKey::from_sec1_bytes(&uncompressed)
            .map_err(|_| CaptchaLikeError::FailedToVerifyCaptcha)?;

        // 5. ECDH → 32-byte shared secret Z
        let scalar = server_sk.to_nonzero_scalar();
        let shared = diffie_hellman(&scalar, epk_pk.as_affine());
        let z_bytes = shared.raw_secret_bytes();

        // 6. ConcatKDF (RFC 7518 §4.6.2) — single SHA-256 round → 32-byte CEK
        //    SHA-256(0x00000001 || Z || len32("A256GCM") || "A256GCM" || 0x00000000 || 0x00000000 || 0x00000100)
        let alg_id = b"A256GCM";
        let mut hasher = Sha256::new();
        hasher.update(1u32.to_be_bytes());
        hasher.update(z_bytes);
        hasher.update((alg_id.len() as u32).to_be_bytes());
        hasher.update(alg_id);
        hasher.update(0u32.to_be_bytes()); // PartyUInfo: empty
        hasher.update(0u32.to_be_bytes()); // PartyVInfo: empty
        hasher.update(256u32.to_be_bytes()); // keydatalen = 256 bits
        let cek = hasher.finalize();

        // 7. AES-256-GCM decrypt (AAD = raw base64url protected header bytes)
        let iv = engine
            .decode(b64_iv)
            .map_err(|_| CaptchaLikeError::FailedToVerifyCaptcha)?;
        let ciphertext = engine
            .decode(b64_ciphertext)
            .map_err(|_| CaptchaLikeError::FailedToVerifyCaptcha)?;
        let tag = engine
            .decode(b64_tag)
            .map_err(|_| CaptchaLikeError::FailedToVerifyCaptcha)?;

        let mut ct_with_tag = ciphertext;
        ct_with_tag.extend_from_slice(&tag);

        let cipher =
            Aes256Gcm::new_from_slice(&cek).map_err(|_| CaptchaLikeError::FailedToVerifyCaptcha)?;
        let nonce = aes_gcm::Nonce::from_slice(&iv);

        cipher
            .decrypt(
                nonce,
                Payload {
                    msg: &ct_with_tag,
                    aad: b64_header.as_bytes(),
                },
            )
            .map_err(|_| {
                log::info!(
                    "Tripwire JWE: AES-GCM decryption failed (tag mismatch or corrupt token)"
                );
                CaptchaLikeError::FailedToVerifyCaptcha
            })
    }
}

#[async_trait::async_trait]
impl CaptchaClient for Layer3IntelTripwireClient {
    async fn verify_captcha(
        &self,
        response: &str,
        ip_addr: &str,
    ) -> Result<CaptchaVerificationOutput, CaptchaVerificationError> {
        // Rule 1: local JWE decryption
        let plaintext = match self.decrypt_jwe(response) {
            Ok(b) => b,
            Err(e) => {
                log::info!("Tripwire: JWE decryption failed: {e:?}");
                return Ok(CaptchaVerificationOutput {
                    result: CaptchaLikeResult::Failure(e),
                    captured_data: None,
                    provider: self.name.clone(),
                });
            }
        };

        let assessment: TripwireAssessment = match serde_json::from_slice(&plaintext) {
            Ok(a) => a,
            Err(e) => {
                log::error!("Tripwire: failed to parse assessment JSON: {e}");
                return Ok(CaptchaVerificationOutput {
                    result: CaptchaLikeResult::Failure(CaptchaLikeError::FailedToVerifyCaptcha),
                    captured_data: None,
                    provider: self.name.clone(),
                });
            }
        };

        let captured_data = self.extract_captured_data(&assessment);

        // Rule 2: proxy check
        if matches!(assessment.proxy, Some(true)) {
            log::info!("Tripwire assessment: {assessment:?}");
            return Ok(CaptchaVerificationOutput {
                result: CaptchaLikeResult::Failure(CaptchaLikeError::AnonymouseAccess),
                captured_data,
                provider: self.name.clone(),
            });
        }

        // Rule 3: IP check (prod only, same guard as MonocleClient)
        if is_prod() {
            let client_reduced = ReducedIpAddr::from(ip_addr.to_string());
            let ip_matches = assessment
                .source_ip
                .as_deref()
                .map(|s| ReducedIpAddr::from(s.to_string()) == client_reduced)
                .unwrap_or(false);
            if !ip_matches {
                log::info!("Tripwire assessment: {assessment:?}");
                return Ok(CaptchaVerificationOutput {
                    result: CaptchaLikeResult::Failure(CaptchaLikeError::FailedToVerifyIpAddress),
                    captured_data,
                    provider: self.name.clone(),
                });
            }
        }

        // Rule 4: timestamp freshness (±300 s)
        if let Some(ts) = assessment.timestamp {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            if (now - ts).abs() > 300 {
                log::info!("Tripwire assessment timestamp out of range: ts={ts}, now={now}");
                return Ok(CaptchaVerificationOutput {
                    result: CaptchaLikeResult::Failure(CaptchaLikeError::FailedToVerifyCaptcha),
                    captured_data,
                    provider: self.name.clone(),
                });
            }
        }

        Ok(CaptchaVerificationOutput {
            result: CaptchaLikeResult::Success,
            captured_data,
            provider: self.name.clone(),
        })
    }
}

pub struct RecaptchaEnterpriseClient {
    name: String,
    client: reqwest::Client,
    api_key: SimpleSecret,
    site_key: String,
    project_id: String,
    score_threshold: f64,
    capture_fields: Vec<String>,
}

impl RecaptchaEnterpriseClient {
    pub fn new(
        name: String,
        api_key: String,
        site_key: String,
        project_id: String,
        score_threshold: f64,
        capture_fields: Vec<String>,
    ) -> Self {
        Self {
            name,
            client: reqwest::Client::new(),
            api_key: SimpleSecret::new(&api_key),
            site_key,
            project_id,
            score_threshold,
            capture_fields,
        }
    }

    fn extract_captured_data(&self, resp: &serde_json::Value) -> Option<serde_json::Value> {
        if self.capture_fields.is_empty() {
            return None;
        }

        let mut captured = serde_json::Map::new();
        for field in &self.capture_fields {
            if let Ok(results) = resp.query(&format!("$.{field}"))
                && let Some(value) = results.first()
            {
                captured.insert(field.clone(), (*value).clone());
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
impl CaptchaClient for RecaptchaEnterpriseClient {
    async fn verify_captcha(
        &self,
        response: &str,
        ip_addr: &str,
    ) -> Result<CaptchaVerificationOutput, CaptchaVerificationError> {
        if self.project_id.is_empty() {
            log::error!(
                "reCAPTCHA Enterprise project_id is not configured for '{}'",
                self.name
            );
            return Ok(CaptchaVerificationOutput {
                result: CaptchaLikeResult::Failure(CaptchaLikeError::FailedToVerifyCaptcha),
                captured_data: None,
                provider: self.name.clone(),
            });
        }

        let url = format!(
            "{}?key={}",
            GRECAPTCHA_ENTERPRISE_URL.replace("{PROJECT_ID}", &self.project_id),
            self.api_key.get()
        );

        let event = GrecaptchaEnterpriseRequest {
            token: response.to_string(),
            site_key: self.site_key.clone(),
            user_agent: String::new(),
            user_ip_address: ip_addr.to_string(),
            ja3: None,
            ja4: None,
            expected_action: GRECAPTCHA_ENTERPRISE_DEFAULT_ACTION.to_string(),
        };

        let res = self
            .client
            .post(&url)
            .json(&serde_json::json!({ "event": event }))
            .send()
            .await
            .map_err(CaptchaVerificationError::Request)?;

        let response_text = res
            .text()
            .await
            .map_err(CaptchaVerificationError::Request)?;

        let resp: GrecaptchaEnterpriseResponse = match serde_json::from_str(&response_text) {
            Ok(v) => v,
            Err(e) => {
                log::error!(
                    "Failed to parse reCAPTCHA Enterprise response: {e}, body: {response_text}"
                );
                return Ok(CaptchaVerificationOutput {
                    result: CaptchaLikeResult::Failure(CaptchaLikeError::FailedToVerifyCaptcha),
                    captured_data: None,
                    provider: self.name.clone(),
                });
            }
        };

        let resp_json = serde_json::to_value(&resp).unwrap_or_default();
        let captured_data = self.extract_captured_data(&resp_json);

        let valid = resp.token_properties.valid;
        let score = resp.risk_analysis.score;

        let result = if valid && score >= self.score_threshold {
            CaptchaLikeResult::Success
        } else {
            log::debug!(
                "reCAPTCHA Enterprise verification failed: valid={valid}, invalid_reason={:?}, score={score}, threshold={}, reasons={:?}",
                resp.token_properties.invalid_reason,
                self.score_threshold,
                resp.risk_analysis.reasons,
            );
            CaptchaLikeResult::Failure(CaptchaLikeError::FailedToVerifyCaptcha)
        };

        Ok(CaptchaVerificationOutput {
            result,
            captured_data,
            provider: self.name.clone(),
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
            if let Ok(results) = resp.query(&format!("$.{field}"))
                && let Some(value) = results.first()
            {
                captured.insert(field.clone(), (*value).clone());
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
        let url = self.config.resolve_placeholders(
            cfg.url.as_deref().unwrap_or_default(),
            response,
            ip_addr,
        );

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

                log::info!(
                    "Sending JSON verification request to {}: {}",
                    self.config.provider,
                    json
                );

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

        log::info!(
            "{} verification response body: {}",
            self.config.provider,
            response_text
        );

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
                    provider: self.config.name.clone(),
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
            provider: self.config.name.clone(),
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
