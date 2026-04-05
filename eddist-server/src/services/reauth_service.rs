use std::collections::HashMap;

use futures::future::join_all;
use metrics::counter;

use crate::{
    domain::captcha_like::CaptchaProviderConfig,
    error::BbsPostAuthWithCodeError,
    external::captcha_like_client::{
        CaptchaLikeError, CaptchaLikeResult, CaptchaVerificationOutput, create_captcha_client,
    },
    repositories::bbs_repository::BbsRepository,
};
use eddist_core::domain::ip_addr::ReducedIpAddr;

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
            let provider_type = config.provider.to_lowercase();
            clients_responses.push((
                create_captcha_client(config, self.redis_conn.clone()),
                (response, input.origin_ip.clone()),
                provider_type,
            ));
        }

        let token = self
            .repo
            .get_authed_token_by_reauth_code(&input.code)
            .await?
            .ok_or_else(|| {
                counter!("reauth_failure", "reason" => "not_found").increment(1);
                BbsPostAuthWithCodeError::FailedToFindAuthedToken
            })?;

        let assert_ip_equality = |token_reduced_ip: ReducedIpAddr, request_origin_ip: &str| {
            let request_origin_reduced = ReducedIpAddr::from(request_origin_ip.to_string());
            if token_reduced_ip != request_origin_reduced {
                counter!("reauth_failure", "reason" => "ip_mismatch").increment(1);
                return Err(BbsPostAuthWithCodeError::FailedToFindAuthedToken);
            }
            Ok(())
        };

        let provider_types = clients_responses
            .iter()
            .map(|x| x.2.clone())
            .collect::<Vec<_>>();
        let handles = clients_responses
            .iter()
            .map(|x| x.0.verify_captcha(&x.1.0, &x.1.1))
            .collect::<Vec<_>>();
        let results = join_all(handles).await;

        let mut has_monocle_style_ip_validation = false;

        for (r, provider_type) in results.into_iter().zip(provider_types.iter()) {
            match r {
                Ok(CaptchaVerificationOutput {
                    result: CaptchaLikeResult::Failure(CaptchaLikeError::FailedToVerifyIpAddress),
                    ..
                }) => assert_ip_equality(token.reduced_ip.clone(), &input.origin_ip)?,
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
                }) => {
                    if provider_type == "monocle" {
                        has_monocle_style_ip_validation = true;
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }

        if !has_monocle_style_ip_validation {
            assert_ip_equality(token.reduced_ip.clone(), &input.origin_ip)?;
        }

        self.repo.clear_require_reauth(token.id).await?;
        counter!("reauth_success").increment(1);

        Ok(())
    }
}

pub struct ReAuthServiceInput {
    pub code: String,
    pub origin_ip: String,
    pub user_agent: String,
    pub asn_num: u32,
    pub captcha_configs: Vec<CaptchaProviderConfig>,
    pub responses: HashMap<String, String>,
}
