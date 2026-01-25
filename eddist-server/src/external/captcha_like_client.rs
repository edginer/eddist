use std::collections::HashMap;

use eddist_core::{domain::ip_addr::ReducedIpAddr, utils::is_prod};
use serde::{Deserialize, Serialize};

use crate::domain::{
    captcha_like::{
        HCaptchaResponse, MonocleResponse, TurnstileResponse, HCAPTCHA_URL, MONOCLE_URL,
        TURNSTILE_URL,
    },
    utils::SimpleSecret,
};

#[async_trait::async_trait]
pub trait CaptchaClient: Send {
    async fn verify_captcha(
        &self,
        response: &str,
        ip_addr: &str,
    ) -> Result<CaptchaLikeResult, reqwest::Error>;
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
    ) -> Result<CaptchaLikeResult, reqwest::Error> {
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
            .await?;

        let response_text = res.text().await?;
        let resp = match serde_json::from_str::<TurnstileResponse>(&response_text) {
            Ok(resp) => resp,
            Err(e) => {
                log::error!(
                    "Failed to parse Turnstile response: {e}, response body: {response_text}"
                );
                return Ok(CaptchaLikeResult::Failure(
                    CaptchaLikeError::FailedToVerifyCaptcha,
                ));
            }
        };

        Ok(if resp.success {
            CaptchaLikeResult::Success
        } else {
            log::info!("Turnstile response: {resp:?}");
            CaptchaLikeResult::Failure(CaptchaLikeError::FailedToVerifyCaptcha)
        })
    }
}

pub struct HCaptchaClient {
    client: reqwest::Client,
    secret: SimpleSecret,
}

impl HCaptchaClient {
    pub fn new(secret: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            secret: SimpleSecret::new(&secret),
        }
    }
}

#[async_trait::async_trait]
impl CaptchaClient for HCaptchaClient {
    async fn verify_captcha(
        &self,
        response: &str,
        ip_addr: &str,
    ) -> Result<CaptchaLikeResult, reqwest::Error> {
        let mut form_data = HashMap::new();
        form_data.insert("response", response);
        form_data.insert("secret", self.secret.get());
        form_data.insert("remoteip", ip_addr);

        let res = self
            .client
            .post(HCAPTCHA_URL)
            .form(&form_data)
            .send()
            .await?;

        let resp = match res.json::<HCaptchaResponse>().await {
            Ok(resp) => resp,
            Err(e) => {
                log::error!("Failed to parse HCaptcha response");
                return Err(e);
            }
        };

        Ok(if resp.success {
            CaptchaLikeResult::Success
        } else {
            log::info!("HCaptcha response: {resp:?}");
            CaptchaLikeResult::Failure(CaptchaLikeError::FailedToVerifyCaptcha)
        })
    }
}

pub struct MonocleClient {
    client: reqwest::Client,
    token: SimpleSecret,
}

impl MonocleClient {
    pub fn new(token: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            token: SimpleSecret::new(&token),
        }
    }
}

#[async_trait::async_trait]
impl CaptchaClient for MonocleClient {
    async fn verify_captcha(
        &self,
        response: &str,
        ip_addr: &str,
    ) -> Result<CaptchaLikeResult, reqwest::Error> {
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
            .await?;

        let resp = match res.json::<MonocleResponse>().await {
            Ok(resp) => resp,
            Err(e) => {
                log::error!("Failed to parse Monocle response");
                return Err(e);
            }
        };

        Ok(if matches!(resp.anon, Some(true)) {
            log::info!("Monocle response: {resp:?}");
            CaptchaLikeResult::Failure(CaptchaLikeError::AnonymouseAccess)
        } else if !verify_ip(resp.ip.as_deref(), resp.ipv6.as_deref()) && is_prod() {
            log::info!("Monocle response: {resp:?}");
            CaptchaLikeResult::Failure(CaptchaLikeError::FailedToVerifyIpAddress)
        } else {
            CaptchaLikeResult::Success
        })
    }
}

#[derive(thiserror::Error, Debug, Serialize, Deserialize)]
pub enum CaptchaLikeError {
    #[error("検証に失敗しました")]
    FailedToVerifyCaptcha,
    #[error("不審な回線からの検証は許可されていません")]
    AnonymouseAccess,
    #[error("IPアドレスの検証に失敗しました")]
    FailedToVerifyIpAddress,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CaptchaLikeResult {
    Success,
    Failure(CaptchaLikeError),
}
