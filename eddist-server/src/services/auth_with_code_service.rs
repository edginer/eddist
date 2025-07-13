use std::collections::HashMap;

use chrono::Utc;
use futures::future::join_all;
use metrics::counter;
use tracing::{error_span, info_span};

use crate::{
    domain::{
        captcha_like::CaptchaLikeConfig,
        service::user_restriction_service::UserRestrictionService,
    },
    error::{BbsCgiError, BbsPostAuthWithCodeError},
    external::captcha_like_client::{
        CaptchaClient, CaptchaLikeResult, HCaptchaClient, MonocleClient, TurnstileClient,
    },
    repositories::{bbs_repository::BbsRepository, user_restriction_repository::UserRestrictionRepository},
};
use eddist_core::domain::ip_addr::ReducedIpAddr;

use super::AppService;

#[derive(Debug, Clone)]
pub struct AuthWithCodeService<T: BbsRepository, R: UserRestrictionRepository>(T, R);

impl<T: BbsRepository, R: UserRestrictionRepository> AuthWithCodeService<T, R> {
    pub fn new(repo: T, user_restriction_repo: R) -> Self {
        Self(repo, user_restriction_repo)
    }
}

#[async_trait::async_trait]
impl<T: BbsRepository, R: UserRestrictionRepository + Clone> AppService<AuthWithCodeServiceInput, AuthWithCodeServiceOutput>
    for AuthWithCodeService<T, R>
{
    async fn execute(
        &self,
        input: AuthWithCodeServiceInput,
    ) -> anyhow::Result<AuthWithCodeServiceOutput> {
        // Check user restriction rules first
        let user_restriction_svc = UserRestrictionService::new(self.1.clone());
        let user_attrs = eddist_core::domain::user_restriction_filter::UserAttributes {
            ip_addr: input.origin_ip.clone(),
            user_agent: input.user_agent.clone(),
            asn_num: input.asn_num,
        };
        if user_restriction_svc.is_user_restricted(&user_attrs, 
            crate::domain::service::user_restriction_service::RestrictionType::AuthCode).await
            .map_err(|e| match e {
                BbsCgiError::Other(anyhow_err) => anyhow_err,
                _ => anyhow::anyhow!("User restriction check failed"),
            })? {
            return Err(BbsPostAuthWithCodeError::UserRestricted.into());
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

        let handles = clients_responses
            .iter()
            .map(|x| x.0.verify_captcha(&x.1 .0, &x.1 .1))
            .collect::<Vec<_>>();
        let results = join_all(handles).await;
        for r in results {
            match r {
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
        let has_monocle = input.captcha_like_configs.iter().any(|config| {
            matches!(config, CaptchaLikeConfig::Monocle { .. })
        });
        
        if !has_monocle {
            let token_origin_ip = ReducedIpAddr::from(token.reduced_ip.clone());
            let request_origin_ip = ReducedIpAddr::from(input.origin_ip.clone());
            
            if token_origin_ip != request_origin_ip {
                counter!("issue_authed_token", "state" => "failed", "reason" => "ip_mismatch")
                    .increment(1);
                return Err(BbsPostAuthWithCodeError::FailedToFindAuthedToken.into());
            }
        }

        self.0
            .activate_authed_status(&token.token, &input.user_agent, now)
            .await?;
        counter!("issue_authed_token", "state" => "success", "source" => "normal").increment(1);

        Ok(AuthWithCodeServiceOutput { token: token.token })
    }
}

pub struct AuthWithCodeServiceInput {
    pub code: String,
    pub origin_ip: String,
    pub user_agent: String,
    pub asn_num: u32,
    pub captcha_like_configs: Vec<CaptchaLikeConfig>,
    pub responses: HashMap<String, String>,
}

pub struct AuthWithCodeServiceOutput {
    pub token: String,
}
