use std::collections::HashMap;

use std::sync::Arc;

use metrics::counter;

use crate::{
    domain::captcha_like::CaptchaProviderConfig,
    error::BbsPostAuthWithCodeError,
    external::captcha_like_client::{
        CaptchaLikeError, CaptchaLikeResult, CaptchaVerificationOutput,
    },
    repositories::bbs_repository::BbsRepository,
    stores::auth_state_store::AuthStateStore,
};

use super::{
    AppService,
    captcha_verification_service::{CaptchaVerificationRunError, CaptchaVerificationService},
};

#[derive(Clone)]
pub struct ReAuthService<T: BbsRepository> {
    repo: T,
    auth_state: Arc<dyn AuthStateStore>,
    captcha: CaptchaVerificationService,
}

impl<T: BbsRepository> ReAuthService<T> {
    pub fn new(
        repo: T,
        auth_state: Arc<dyn AuthStateStore>,
        captcha: CaptchaVerificationService,
    ) -> Self {
        Self {
            repo,
            auth_state,
            captcha,
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

        let captcha = match self
            .captcha
            .prepare(&input.captcha_configs, &input.responses)
        {
            Ok(captcha) => captcha,
            Err(CaptchaVerificationRunError::MissingResponse) => {
                counter!("reauth_failure", "reason" => "missing_captcha_response").increment(1);
                return Err(BbsPostAuthWithCodeError::CaptchaError(
                    CaptchaLikeError::FailedToVerifyCaptcha,
                )
                .into());
            }
            Err(CaptchaVerificationRunError::Client(e)) => return Err(e.into()),
        };
        let verifications = captcha.run(&input.origin_ip).await;

        // Unlike auth-code, re-auth rejects any provider failure as-is; there is no
        // token IP fallback for providers whose IP comparison failed.
        for verification in verifications {
            match verification.result {
                Ok(CaptchaVerificationOutput {
                    result: CaptchaLikeResult::Failure(e),
                    ..
                }) => {
                    counter!("reauth_failure", "reason" => format!("captcha_{}", verification.provider_type))
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
