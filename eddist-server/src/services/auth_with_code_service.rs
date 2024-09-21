use std::collections::HashMap;

use chrono::Utc;
use eddist_core::domain::ip_addr::{IpAddr, ReducedIpAddr};
use futures::future::join_all;
use tracing::{error_span, info_span};

use crate::{
    domain::{captcha_like::CaptchaLikeConfig, res},
    external::captcha_like_client::{
        CaptchaClient, CaptchaLikeResult, HCaptchaClient, MonocleClient, TurnstileClient,
    },
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
        let clients_responses = input
            .captcha_like_configs
            .iter()
            .filter_map(|config| {
                let client: Box<dyn CaptchaClient> = match config {
                    CaptchaLikeConfig::Turnstile { secret, .. } => {
                        Box::new(TurnstileClient::new(secret.clone()))
                    }
                    CaptchaLikeConfig::Hcaptcha { site_key, secret } => {
                        Box::new(HCaptchaClient::new(site_key.clone(), secret.clone()))
                    }
                    CaptchaLikeConfig::Monocle { token, .. } => {
                        Box::new(MonocleClient::new(token.clone()))
                    }
                    _ => {
                        error_span!("unsupported captcha like config, ignored", config = ?config);
                        return None;
                    }
                };
                let req_params = match config {
                    CaptchaLikeConfig::Turnstile { .. } => (
                        input.responses["cf-turnstile-response"].to_string(),
                        input.origin_ip.clone(),
                    ),
                    CaptchaLikeConfig::Hcaptcha { .. } => (
                        input.responses["h-captcha-response"].to_string(),
                        input.origin_ip.clone(),
                    ),
                    CaptchaLikeConfig::Monocle { .. } => (
                        input.responses["monocle"].to_string(),
                        input.origin_ip.clone(),
                    ),
                    _ => unreachable!(),
                };
                Some((client, req_params))
            })
            .collect::<Vec<_>>();

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

        let handles = clients_responses
            .iter()
            .map(|x| x.0.verify_captcha(&x.1 .0, &x.1 .1))
            .collect::<Vec<_>>();
        let results = join_all(handles).await;
        for r in results {
            match r {
                Ok(CaptchaLikeResult::Failure(e)) => {
                    return Err(e.into());
                }
                Err(e) => return Err(e.into()),
                _ => {}
            }
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
