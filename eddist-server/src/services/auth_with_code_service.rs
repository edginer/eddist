use std::collections::HashMap;

use chrono::Utc;
use futures::future::join_all;
use metrics::counter;
use tracing::{error_span, info_span};

use crate::{
    domain::captcha_like::CaptchaLikeConfig,
    error::BbsPostAuthWithCodeError,
    external::captcha_like_client::{
        CapClient, CaptchaClient, CaptchaLikeError, CaptchaLikeResult, HCaptchaClient,
        MonocleClient, TurnstileClient,
    },
    repositories::bbs_repository::BbsRepository,
};
use eddist_core::domain::ip_addr::ReducedIpAddr;
use uuid::Uuid;

use super::AppService;

#[derive(Debug, Clone)]
pub struct AuthWithCodeService<T: BbsRepository>(T);

impl<T: BbsRepository> AuthWithCodeService<T> {
    pub fn new(repo: T) -> Self {
        Self(repo)
    }

    /// Generate a new rate limiting token (browser handles expiration via Max-Age)
    fn generate_rate_limit_token(&self) -> String {
        Uuid::now_v7().to_string().replace("-", "")
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
        if input.rate_limit_token.is_some() {
            // User is rate limited (cookie exists and browser hasn't expired it)
            counter!("issue_authed_token", "state" => "failed", "reason" => "rate_limited")
                .increment(1);
            return Err(BbsPostAuthWithCodeError::RateLimited.into());
        }
        let clients_responses = input
            .captcha_like_configs
            .iter()
            .filter_map(|config| {
                let client: Box<dyn CaptchaClient> = match config {
                    CaptchaLikeConfig::Turnstile { secret, .. } => {
                        Box::new(TurnstileClient::new(secret.clone()))
                    }
                    CaptchaLikeConfig::Hcaptcha { secret, .. } => {
                        Box::new(HCaptchaClient::new(secret.clone()))
                    }
                    CaptchaLikeConfig::Monocle { token, .. } => {
                        Box::new(MonocleClient::new(token.clone()))
                    }
                    CaptchaLikeConfig::Cap {
                        base_url,
                        site_key,
                        secret,
                    } => Box::new(CapClient::new(
                        base_url.clone(),
                        site_key.clone(),
                        secret.clone(),
                    )),
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
                    CaptchaLikeConfig::Cap { .. } => (
                        input.responses["cap-token"].to_string(),
                        input.origin_ip.clone(),
                    ),
                    _ => unreachable!(),
                };
                Some((client, req_params))
            })
            .collect::<Vec<_>>();
        counter!("issue_authed_token", "state" => "request").increment(1);

        // Get all unauthed tokens with the auth code (non-IP checking)
        let candidate_tokens = self
            .0
            .get_unauthed_authed_token_by_auth_code(&input.code)
            .await?;

        // Filter out already authed tokens (only check unauthed tokens)
        let unauthed_tokens: Vec<_> = candidate_tokens
            .into_iter()
            .filter(|token| token.authed_at.is_none()) // Only include tokens that are not yet authed
            .collect();

        // Handle auth_code collisions - if multiple unauthed tokens exist, delete them and return error
        if unauthed_tokens.len() > 1 {
            // Delete all unauthed tokens with this auth_code to resolve collision
            for candidate in &unauthed_tokens {
                self.0.delete_authed_token(&candidate.token).await?;
            }
            counter!("issue_authed_token", "state" => "failed", "reason" => "auth_code_collision")
                .increment(1);
            return Err(BbsPostAuthWithCodeError::AuthCodeCollision.into());
        }

        // Check if we found exactly one unauthed token
        let Some(token) = unauthed_tokens.into_iter().next() else {
            counter!("issue_authed_token", "state" => "failed", "reason" => "not_found")
                .increment(1);
            info_span!("failed to find authed token", code = %input.code);
            return Err(BbsPostAuthWithCodeError::FailedToFindAuthedToken.into());
        };

        let now = Utc::now();
        if token.is_activation_expired(now) {
            counter!("issue_authed_token", "state" => "failed", "reason" => "expired").increment(1);
            return Err(BbsPostAuthWithCodeError::ExpiredActivationCode.into());
        }

        let assert_ip_equality = |token_reduced_ip: ReducedIpAddr, request_origin_ip: &str| {
            let request_origin_reduced = ReducedIpAddr::from(request_origin_ip.to_string());

            if token_reduced_ip != request_origin_reduced {
                counter!("issue_authed_token", "state" => "failed", "reason" => "ip_mismatch")
                    .increment(1);
                return Err(BbsPostAuthWithCodeError::FailedToFindAuthedToken);
            }

            Ok(())
        };

        let handles = clients_responses
            .iter()
            .map(|x| x.0.verify_captcha(&x.1 .0, &x.1 .1))
            .collect::<Vec<_>>();
        let results = join_all(handles).await;
        for r in results {
            match r {
                // CaptchaLikeError::FailedToVerifyIpAddress is only used for spur.us (currently)
                Ok(CaptchaLikeResult::Failure(CaptchaLikeError::FailedToVerifyIpAddress)) => {
                    assert_ip_equality(token.reduced_ip.clone(), &input.origin_ip)?
                }
                Ok(CaptchaLikeResult::Failure(e)) => {
                    counter!("issue_authed_token", "state" => "failed", "reason" => "captcha")
                        .increment(1);
                    return Err(BbsPostAuthWithCodeError::CaptchaError(e).into());
                }
                Err(e) => return Err(e.into()),
                _ => {}
            }
        }

        // Check IP equality for users not using spur.us (Monocle already handles IPv4/IPv6 checking)
        let has_monocle = input
            .captcha_like_configs
            .iter()
            .any(|config| matches!(config, CaptchaLikeConfig::Monocle { .. }));

        if !has_monocle {
            assert_ip_equality(token.reduced_ip.clone(), &input.origin_ip)?;
        }

        self.0
            .activate_authed_status(&token.token, &input.user_agent, now)
            .await?;
        counter!("issue_authed_token", "state" => "success", "source" => "normal").increment(1);

        // Generate rate limiting token after successful authentication
        let rate_limit_token = Some(self.generate_rate_limit_token());

        Ok(AuthWithCodeServiceOutput {
            token: token.token,
            rate_limit_token,
        })
    }
}

pub struct AuthWithCodeServiceInput {
    pub code: String,
    pub origin_ip: String,
    pub user_agent: String,
    pub captcha_like_configs: Vec<CaptchaLikeConfig>,
    pub responses: HashMap<String, String>,
    pub rate_limit_token: Option<String>,
}

pub struct AuthWithCodeServiceOutput {
    pub token: String,
    pub rate_limit_token: Option<String>,
}
