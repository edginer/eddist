use std::collections::HashMap;

use chrono::Utc;
use eddist_core::domain::ip_addr::{IpAddr, ReducedIpAddr};
use tracing::info_span;

use crate::{
    domain::captcha_like::CaptchaLikeConfig, external::captcha_like_client::TurnstileClient,
    repositories::bbs_repository::BbsRepository,
};

use super::AppService;

#[derive(Debug, Clone)]
pub struct AuthWithCodeService<T: BbsRepository>(T);

impl<T: BbsRepository> AuthWithCodeService<T> {
    pub fn new(repo: T) -> Self {
        Self(repo)
    }
}

#[async_trait::async_trait]
impl<T: BbsRepository> AppService<AuthWithCodeServiceInput, AuthWithCodeServiceOutput>
    for AuthWithCodeService<T>
{
    async fn execute(
        &self,
        input: AuthWithCodeServiceInput,
    ) -> anyhow::Result<AuthWithCodeServiceOutput> {
        let CaptchaLikeConfig::Turnstile { secret, .. } = &input.captcha_like_configs[0] else {
            return Err(anyhow::anyhow!("failed to find turnstile config"));
        };

        let ip_addr = IpAddr::new(input.origin_ip.clone());
        let reduced = ReducedIpAddr::from(ip_addr.clone());
        let Some(token) = self
            .0
            .get_authed_token_by_origin_ip_and_auth_code(&reduced.to_string(), &input.code)
            .await?
        else {
            info_span!("failed to find authed token", reduced_ip = %reduced.to_string(), origin_ip = %ip_addr, code = %input.code);
            return Err(anyhow::anyhow!("failed to find authed token"));
        };

        if token.validity {
            return Err(anyhow::anyhow!("authed token is already valid"));
        }

        let now = Utc::now();
        if token.is_activation_expired(now) {
            return Err(anyhow::anyhow!("activation code is expired"));
        }

        let turnstile_client = TurnstileClient::new(secret.clone());
        let result = turnstile_client
            .verify_captcha(
                &input.responses["cf-turnstile-response"].to_string(),
                &input.origin_ip,
            )
            .await?;

        if !result {
            return Err(anyhow::anyhow!("failed to verify captcha"));
        }

        self.0
            .activate_authed_status(&token.token, &input.user_agent, now)
            .await?;

        Ok(AuthWithCodeServiceOutput { token: token.token })
    }
}

pub struct AuthWithCodeServiceInput {
    pub code: String,
    pub origin_ip: String,
    pub user_agent: String,
    pub captcha_like_configs: Vec<CaptchaLikeConfig>,
    pub responses: HashMap<String, String>,
}

pub struct AuthWithCodeServiceOutput {
    pub token: String,
}
