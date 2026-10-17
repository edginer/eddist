use std::collections::HashMap;

use std::sync::Arc;

use futures::future::join_all;
use metrics::counter;

use crate::{
    domain::captcha_like::CaptchaProviderConfig,
    error::BbsPostAuthWithCodeError,
    external::captcha_like_client::{
        CaptchaLikeError, CaptchaLikeResult, CaptchaVerificationOutput, create_captcha_client,
    },
    repositories::bbs_repository::BbsRepository,
    stores::auth_state_store::AuthStateStore,
};

use super::AppService;

#[derive(Clone)]
pub struct ReAuthService<T: BbsRepository> {
    repo: T,
    auth_state: Arc<dyn AuthStateStore>,
    redis_conn: redis::aio::ConnectionManager,
}

impl<T: BbsRepository> ReAuthService<T> {
    pub fn new(
        repo: T,
        auth_state: Arc<dyn AuthStateStore>,
        redis_conn: redis::aio::ConnectionManager,
    ) -> Self {
        Self {
            repo,
            auth_state,
            redis_conn,
        }
    }
}

#[async_trait::async_trait]
impl<T: BbsRepository> AppService<ReAuthServiceInput, ()> for ReAuthService<T> {
    async fn execute(&self, input: ReAuthServiceInput) -> anyhow::Result<()> {
        let token_id = self
            .auth_state
            .consume_reauth_code(&input.temp_key)
            .await
            .ok_or_else(|| {
                counter!("reauth_failure", "reason" => "invalid_temp_key").increment(1);
                BbsPostAuthWithCodeError::FailedToFindAuthedToken
            })?;

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
                    tracing::error!(
                        provider = %config.provider,
                        field = %form_field_name,
                        "captcha response not found in form"
                    );
                    return Err(BbsPostAuthWithCodeError::CaptchaError(
                        CaptchaLikeError::FailedToVerifyCaptcha,
                    )
                    .into());
                }
            };
            clients_responses.push((
                create_captcha_client(config, self.redis_conn.clone())?,
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
