use std::collections::HashMap;

use futures::future::join_all;
use metrics::counter;
use redis::AsyncCommands;
use uuid::Uuid;

use crate::{
    domain::captcha_like::CaptchaProviderConfig,
    error::BbsPostAuthWithCodeError,
    external::captcha_like_client::{
        CaptchaLikeError, CaptchaLikeResult, CaptchaVerificationOutput, create_captcha_client,
    },
    repositories::bbs_repository::BbsRepository,
};
use eddist_core::redis_keys::{reauth_lock_key, reauth_temp_key};

use super::AppService;

#[derive(Clone)]
pub struct ReAuthService<T: BbsRepository> {
    repo: T,
    redis_conn: redis::aio::ConnectionManager,
}

impl<T: BbsRepository> ReAuthService<T> {
    pub fn new(repo: T, redis_conn: redis::aio::ConnectionManager) -> Self {
        Self { repo, redis_conn }
    }
}

#[async_trait::async_trait]
impl<T: BbsRepository> AppService<ReAuthServiceInput, ()> for ReAuthService<T> {
    async fn execute(&self, input: ReAuthServiceInput) -> anyhow::Result<()> {
        let redis_key = reauth_temp_key(&input.temp_key);
        // Atomically read and delete the temp key — one-time use, closes replay window
        let mut conn = self.redis_conn.clone();
        let token_id_str: Option<String> = conn.get_del(&redis_key).await.unwrap_or(None);
        let token_id_str = token_id_str.ok_or_else(|| {
            counter!("reauth_failure", "reason" => "invalid_temp_key").increment(1);
            BbsPostAuthWithCodeError::FailedToFindAuthedToken
        })?;
        let token_id = Uuid::parse_str(&token_id_str).map_err(|_| {
            counter!("reauth_failure", "reason" => "invalid_temp_key").increment(1);
            BbsPostAuthWithCodeError::FailedToFindAuthedToken
        })?;
        // Delete the lock key so new codes can be issued after successful re-auth
        let _ = conn.del::<_, ()>(&reauth_lock_key(&token_id_str)).await;

        let token = self
            .repo
            .get_authed_token_by_id(token_id)
            .await?
            .filter(|t| t.require_reauth && t.validity)
            .ok_or_else(|| {
                counter!("reauth_failure", "reason" => "not_found").increment(1);
                BbsPostAuthWithCodeError::FailedToFindAuthedToken
            })?;

        let mut clients_responses = Vec::new();
        for config in &input.captcha_configs {
            let form_field_name = &config.widget.form_field_name;
            let response = match input.responses.get(form_field_name) {
                Some(r) => r.clone(),
                None => {
                    counter!("reauth_failure", "reason" => "missing_captcha_response").increment(1);
                    return Err(BbsPostAuthWithCodeError::CaptchaError(
                        CaptchaLikeError::FailedToVerifyCaptcha,
                    )
                    .into());
                }
            };
            clients_responses.push((
                create_captcha_client(config, self.redis_conn.clone()),
                (response, input.origin_ip.clone()),
                config.provider.to_lowercase(),
            ));
        }

        let provider_types = clients_responses
            .iter()
            .map(|x| x.2.clone())
            .collect::<Vec<_>>();
        let handles = clients_responses
            .iter()
            .map(|x| x.0.verify_captcha(&x.1.0, &x.1.1))
            .collect::<Vec<_>>();
        let results = join_all(handles).await;

        for (r, provider_type) in results.into_iter().zip(provider_types.iter()) {
            match r {
                Ok(CaptchaVerificationOutput {
                    result: CaptchaLikeResult::Failure(e),
                    ..
                }) => {
                    counter!("reauth_failure", "reason" => format!("captcha_{provider_type}"))
                        .increment(1);
                    return Err(BbsPostAuthWithCodeError::CaptchaError(e).into());
                }
                Ok(CaptchaVerificationOutput {
                    result: CaptchaLikeResult::Success,
                    ..
                }) => {}
                Err(e) => return Err(e.into()),
            }
        }

        self.repo.clear_require_reauth(token.id).await?;
        counter!("reauth_success").increment(1);

        Ok(())
    }
}

pub struct ReAuthServiceInput {
    pub temp_key: String,
    pub origin_ip: String,
    pub captcha_configs: Vec<CaptchaProviderConfig>,
    pub responses: HashMap<String, String>,
}
