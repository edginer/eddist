use std::collections::HashMap;

use eddist_core::domain::ip_addr::ReducedIpAddr;
use serde::{Deserialize, Serialize};

use crate::domain::{
    captcha_like::{
        HCaptchaResponse, MonocleResponse, TurnstileResponse, HCAPTCHA_URL, MONOCLE_URL,
        TURNSTILE_URL,
    },
    utils::SimpleSecret,
};

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

    pub async fn verify_captcha(
        &self,
        response: &str,
        ip_addr: &str,
    ) -> Result<CaptchaLikeResult, reqwest::Error> {
        let mut form_data = HashMap::new();
        form_data.insert("response", response);
        form_data.insert("remoteip", ip_addr);
        form_data.insert("remoteip_leniency", "strict");
        form_data.insert("secret", self.secret.get());

        let res = self
            .client
            .post(TURNSTILE_URL)
            .header("Authorization", self.secret.get())
            .form(&form_data)
            .send()
            .await?;

        let resp = res.json::<TurnstileResponse>().await?;

        Ok(if resp.success {
            CaptchaLikeResult::Success
        } else {
            CaptchaLikeResult::Failure(CaptchaLikeError::FailedToVerifyCaptcha)
        })
    }
}

pub struct HCaptchaClient {
    client: reqwest::Client,
    secret: SimpleSecret,
    site_key: String,
}

impl HCaptchaClient {
    pub fn new(secret: String, site_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            secret: SimpleSecret::new(&secret),
            site_key,
        }
    }

    pub async fn verify_captcha(
        &self,
        response: &str,
        ip_addr: &str,
    ) -> Result<CaptchaLikeResult, reqwest::Error> {
        let mut form_data = HashMap::new();
        form_data.insert("response", response);
        form_data.insert("secret", self.secret.get());
        form_data.insert("sitekey", &self.site_key);
        form_data.insert("remoteip", ip_addr);

        let res = self
            .client
            .post(HCAPTCHA_URL)
            .form(&form_data)
            .send()
            .await?;

        let resp = res.json::<HCaptchaResponse>().await?;

        Ok(if resp.success {
            CaptchaLikeResult::Success
        } else {
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

    pub async fn verify_captcha(
        &self,
        response: &str,
        ip_addr: &str,
    ) -> Result<CaptchaLikeResult, reqwest::Error> {
        let response = response.to_string();

        let verify_ip = |v4ip: Option<&str>, v6ip: Option<&str>| {
            let from_client_ip_addr = ReducedIpAddr::from(ip_addr.to_string());

            let monocle_ip = if from_client_ip_addr.is_v4() {
                let Some(monocle_v4ip) = v4ip else {
                    return false;
                };
                monocle_v4ip
            } else {
                let Some(monocle_v6ip) = v6ip else {
                    return false;
                };
                monocle_v6ip
            };

            from_client_ip_addr == ReducedIpAddr::from(monocle_ip.to_string())
        };

        let res = self
            .client
            .post(MONOCLE_URL)
            .header("Content-Type", "text/plain; charset=utf-8")
            .header("TOKEN", self.token.get())
            .body(response)
            .send()
            .await?;

        let resp = res.json::<MonocleResponse>().await?;

        Ok(if matches!(resp.anon, Some(true)) {
            CaptchaLikeResult::Failure(CaptchaLikeError::AnonymouseAccess)
        } else if !verify_ip(resp.ip.as_deref(), resp.ipv6.as_deref()) {
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

impl CaptchaLikeResult {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }
}
