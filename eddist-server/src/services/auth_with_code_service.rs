use std::collections::HashMap;

use chrono::Utc;
use futures::future::join_all;
use metrics::counter;
use redis::{AsyncCommands, aio::ConnectionManager};
use tracing::{error_span, info_span};

use crate::{
    domain::{
        captcha_like::CaptchaProviderConfig,
        user::user_reg_state::{RegistrationSource, TempUrlRegistrationRecord},
    },
    error::BbsPostAuthWithCodeError,
    external::captcha_like_client::{
        CaptchaLikeError, CaptchaLikeResult, CaptchaVerificationOutput, create_captcha_client,
    },
    repositories::{bbs_pubsub_repository::CreationEventRepository, bbs_repository::BbsRepository},
    utils::redis::user_reg_temp_url_register_key,
};
use eddist_core::{
    domain::{
        ip_addr::ReducedIpAddr,
        pubsub_repository::{AuthTokenRequested, AuthTokenSucceeded},
    },
    utils::is_auth_token_pub_enabled,
};
use rand::{Rng, distr::Uniform};
use uuid::Uuid;

use super::{
    AppService,
    server_settings_cache::{ServerSettingKey, get_server_setting_bool},
};

const USER_REG_TEMP_URL_LEN: usize = 5;

#[derive(Clone)]
pub struct AuthWithCodeService<T: BbsRepository, E: CreationEventRepository> {
    repo: T,
    redis_conn: ConnectionManager,
    event_repo: E,
}

impl<T: BbsRepository, E: CreationEventRepository> AuthWithCodeService<T, E> {
    pub fn new(repo: T, redis_conn: ConnectionManager, event_repo: E) -> Self {
        Self {
            repo,
            redis_conn,
            event_repo,
        }
    }

    /// Generate a new rate limiting token (browser handles expiration via Max-Age)
    fn generate_rate_limit_token(&self) -> String {
        Uuid::now_v7().to_string().replace("-", "")
    }
}

#[async_trait::async_trait]
impl<T: BbsRepository, E: CreationEventRepository>
    AppService<AuthWithCodeServiceInput, AuthWithCodeServiceOutput> for AuthWithCodeService<T, E>
{
    async fn execute(
        &self,
        input: AuthWithCodeServiceInput,
    ) -> anyhow::Result<AuthWithCodeServiceOutput> {
        if input.rate_limit_token.is_some() {
            // User is rate limited (cookie exists and browser hasn't expired it)
            counter!("auth_code_failure", "reason" => "rate_limited").increment(1);
            return Err(BbsPostAuthWithCodeError::RateLimited.into());
        }
        let clients_responses = input
            .captcha_like_configs
            .iter()
            .filter_map(|config| {
                // Get the form field name from config
                let form_field_name = &config.widget.form_field_name;

                // Get the response for this captcha from the form data
                let response = match input.responses.get(form_field_name) {
                    Some(r) => r.clone(),
                    None => {
                        error_span!(
                            "captcha response not found in form",
                            provider = %config.provider,
                            field = %form_field_name
                        );
                        return None;
                    }
                };

                let provider_type = config.provider.to_lowercase();
                Some((
                    create_captcha_client(config),
                    (response, input.origin_ip.clone()),
                    provider_type,
                ))
            })
            .collect::<Vec<_>>();
        counter!("auth_code_request").increment(1);
        if is_auth_token_pub_enabled() {
            let event_repo = self.event_repo.clone();
            let event = AuthTokenRequested {
                origin_ip: input.origin_ip.clone(),
                user_agent: input.user_agent.clone(),
                asn_num: input.asn_num,
                auth_code: input.code.clone(),
            };
            tokio::spawn(async move {
                let _ = event_repo.publish_auth_token_requested(event).await;
            });
        }

        // Get all unauthed tokens with the auth code (non-IP checking)
        let candidate_tokens = self
            .repo
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
                self.repo.delete_authed_token(&candidate.token).await?;
            }
            counter!("auth_code_failure", "reason" => "auth_code_collision").increment(1);
            return Err(BbsPostAuthWithCodeError::AuthCodeCollision.into());
        }

        // Check if we found exactly one unauthed token
        let Some(token) = unauthed_tokens.into_iter().next() else {
            counter!("auth_code_failure", "reason" => "not_found").increment(1);
            info_span!("failed to find authed token", code = %input.code);
            return Err(BbsPostAuthWithCodeError::FailedToFindAuthedToken.into());
        };

        let now = Utc::now();
        if token.is_activation_expired(now) {
            counter!("auth_code_failure", "reason" => "expired").increment(1);
            return Err(BbsPostAuthWithCodeError::ExpiredActivationCode.into());
        }

        let assert_ip_equality = |token_reduced_ip: ReducedIpAddr, request_origin_ip: &str| {
            let request_origin_reduced = ReducedIpAddr::from(request_origin_ip.to_string());

            if token_reduced_ip != request_origin_reduced {
                counter!("auth_code_failure", "reason" => "ip_mismatch").increment(1);
                return Err(BbsPostAuthWithCodeError::FailedToFindAuthedToken);
            }

            Ok(())
        };

        let provider_types: Vec<String> = clients_responses.iter().map(|x| x.2.clone()).collect();
        let handles = clients_responses
            .iter()
            .map(|x| x.0.verify_captcha(&x.1.0, &x.1.1))
            .collect::<Vec<_>>();
        let results = join_all(handles).await;

        // Collect captured data from all verification results
        let mut captured_data_map = HashMap::<String, serde_json::Value>::new();
        let mut has_monocle_style_ip_validation = false;

        for (r, provider_type) in results.into_iter().zip(provider_types.iter()) {
            match r {
                Ok(CaptchaVerificationOutput {
                    result: CaptchaLikeResult::Failure(CaptchaLikeError::FailedToVerifyIpAddress),
                    captured_data,
                    provider,
                }) => {
                    // Store captured data even on IP verification failure
                    if let Some(data) = captured_data {
                        captured_data_map.insert(provider.clone(), data);
                    }
                    // IP verification failed, fall back to token IP check
                    assert_ip_equality(token.reduced_ip.clone(), &input.origin_ip)?
                }
                Ok(CaptchaVerificationOutput {
                    result: CaptchaLikeResult::Failure(e),
                    captured_data,
                    provider,
                }) => {
                    // Store captured data even on failure
                    if let Some(data) = captured_data {
                        captured_data_map.insert(provider, data);
                    }
                    counter!("auth_code_failure", "reason" => format!("captcha_{provider_type}"))
                        .increment(1);
                    return Err(BbsPostAuthWithCodeError::CaptchaError(e).into());
                }
                Ok(CaptchaVerificationOutput {
                    result: CaptchaLikeResult::Success,
                    captured_data,
                    provider,
                }) => {
                    // Store captured data on success
                    if let Some(data) = captured_data {
                        captured_data_map.insert(provider.clone(), data);
                    }
                    // Track if provider has built-in IP validation (Monocle)
                    if provider_type == "monocle" {
                        has_monocle_style_ip_validation = true;
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }

        // Check IP equality for users not using a provider with built-in IP validation
        if !has_monocle_style_ip_validation {
            assert_ip_equality(token.reduced_ip.clone(), &input.origin_ip)?;
        }

        // Build additional_info JSON with captured data
        let additional_info = if captured_data_map.is_empty() {
            None
        } else {
            Some(serde_json::json!({
                "captcha_verification": captured_data_map,
                "verified_at": now.to_rfc3339(),
            }))
        };

        self.repo
            .activate_authed_status(
                &token.token,
                &input.user_agent,
                now,
                additional_info.clone(),
            )
            .await?;
        counter!("auth_code_success").increment(1);
        if is_auth_token_pub_enabled() {
            let event_repo = self.event_repo.clone();
            let event = AuthTokenSucceeded {
                authed_token_id: token.id,
                origin_ip: input.origin_ip.clone(),
                user_agent: input.user_agent.clone(),
                asn_num: token.asn_num as u32,
                authed_at: now,
                additional_info,
            };
            tokio::spawn(async move {
                let _ = event_repo.publish_auth_token_succeeded(event).await;
            });
        }

        // Generate rate limiting token after successful authentication
        let rate_limit_token = Some(self.generate_rate_limit_token());

        // Generate a temp URL for optional IdP linking if enabled
        let user_reg_url = if get_server_setting_bool(ServerSettingKey::EnableIdpLinking).await {
            let mut redis_conn = self.redis_conn.clone();
            let temp_url_path = generate_random_string(USER_REG_TEMP_URL_LEN);
            redis_conn
                .set_ex::<_, _, ()>(
                    user_reg_temp_url_register_key(&temp_url_path),
                    serde_json::to_string(&TempUrlRegistrationRecord {
                        authed_token_id: token.id.to_string(),
                        source: RegistrationSource::AuthCode,
                    })?,
                    60 * 3, // 3 minutes TTL
                )
                .await?;
            Some(format!("/user/register/{temp_url_path}"))
        } else {
            None
        };

        Ok(AuthWithCodeServiceOutput {
            token: token.token,
            authed_token_id: token.id,
            rate_limit_token,
            user_reg_url,
        })
    }
}

pub struct AuthWithCodeServiceInput {
    pub code: String,
    pub origin_ip: String,
    pub user_agent: String,
    pub asn_num: u32,
    pub captcha_like_configs: Vec<CaptchaProviderConfig>,
    pub responses: HashMap<String, String>,
    pub rate_limit_token: Option<String>,
}

pub struct AuthWithCodeServiceOutput {
    pub token: String,
    pub authed_token_id: Uuid,
    pub rate_limit_token: Option<String>,
    /// URL path for optional user registration/IdP linking (only set when IdP linking is enabled)
    pub user_reg_url: Option<String>,
}

fn generate_random_string(len: usize) -> String {
    let charset: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZabcdefghjkmnpqrstuvwxyz";
    let index_dist = Uniform::try_from(0..charset.len()).unwrap();
    (0..len)
        .map(|_| {
            let idx = rand::rng().sample(index_dist);
            charset[idx] as char
        })
        .collect()
}
