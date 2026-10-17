use std::sync::Arc;

use chrono::Utc;
use eddist_core::simple_rate_limiter::RateLimiter;
use metrics::counter;
use tokio::sync::Mutex;

use crate::{
    config::AppConfig,
    domain::{
        authed_token::AuthedToken,
        service::bbscgi_user_reg_temp_url_service::{UserRegUrlKind, create_userreg_temp_url},
    },
    error::BbsCgiError,
    repositories::{
        bbs_pubsub_repository::{CreationEventRepository, record_event_publish},
        bbs_repository::{BbsRepository, CreatingAuthedToken},
    },
    services::server_settings_cache::{ServerSettingKey, ServerSettingsSnapshot},
    stores::auth_state_store::AuthStateStore,
    tasks::BackgroundTasks,
};
use eddist_core::{
    domain::pubsub_repository::AuthTokenInitiated, utils::is_auth_token_pub_enabled,
};

#[derive(Clone)]
pub struct UserCreationRateLimit(Arc<Mutex<RateLimiter>>);

impl Default for UserCreationRateLimit {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(RateLimiter::new(
            5,
            std::time::Duration::from_secs(60 * 60),
        ))))
    }
}

impl UserCreationRateLimit {
    pub async fn check_and_add(&self, token: &str) -> bool {
        self.0.lock().await.check_and_add(token)
    }
}

#[derive(Clone)]
pub struct BbsCgiAuthService<T: BbsRepository, E: CreationEventRepository> {
    repo: T,
    auth_state: Arc<dyn AuthStateStore>,
    event_repo: E,
    config: Arc<AppConfig>,
    user_creation_rate_limit: UserCreationRateLimit,
    background_tasks: BackgroundTasks,
}

impl<T: BbsRepository, E: CreationEventRepository> BbsCgiAuthService<T, E> {
    pub fn new(
        repo: T,
        auth_state: Arc<dyn AuthStateStore>,
        event_repo: E,
        config: Arc<AppConfig>,
        user_creation_rate_limit: UserCreationRateLimit,
        background_tasks: BackgroundTasks,
    ) -> Self {
        Self {
            repo,
            auth_state,
            event_repo,
            config,
            user_creation_rate_limit,
            background_tasks,
        }
    }

    fn publish_initiated(
        &self,
        authed_token_id: uuid::Uuid,
        origin_ip: String,
        user_agent: String,
        asn_num: u32,
    ) {
        if is_auth_token_pub_enabled() {
            let event_repo = self.event_repo.clone();
            self.background_tasks
                .spawn_detached("auth_token_initiated_publish", async move {
                    record_event_publish(
                        "auth_token_initiated",
                        event_repo
                            .publish_auth_token_initiated(AuthTokenInitiated {
                                authed_token_id,
                                origin_ip,
                                user_agent,
                                asn_num,
                            })
                            .await,
                    );
                });
        }
    }

    async fn issue_auth_code(
        &self,
        ip_addr: String,
        user_agent: String,
        asn_num: i32,
        created_at: chrono::DateTime<chrono::Utc>,
        settings: &ServerSettingsSnapshot,
    ) -> Result<AuthedToken, BbsCgiError> {
        if settings.is_enabled(ServerSettingKey::CloseNewAuthentication) {
            return Err(BbsCgiError::NewAuthenticationClosed);
        }
        let require_user_registration = settings.is_enabled(ServerSettingKey::RequireIdpLinking);

        let authed_token = AuthedToken::new(ip_addr.clone(), user_agent.clone(), asn_num);
        self.repo
            .create_authed_token(CreatingAuthedToken {
                token: authed_token.token.clone(),
                writing_ua: authed_token.writing_ua,
                origin_ip: authed_token.origin_ip,
                asn_num: authed_token.asn_num,
                created_at,
                author_id_seed: authed_token.author_id_seed,
                auth_code: authed_token.auth_code.clone(),
                id: authed_token.id,
                require_user_registration,
            })
            .await?;
        counter!("token_request", "state" => "created").increment(1);
        self.publish_initiated(authed_token.id, ip_addr, user_agent, asn_num as u32);

        Err(BbsCgiError::Unauthenticated {
            auth_code: authed_token.auth_code,
            base_url: self.config.base_url.clone(),
            auth_token: authed_token.token,
        })
    }

    pub async fn check_validity(
        &self,
        token: Option<&str>,
        ip_addr: String,
        user_agent: String,
        asn_num: i32,
        created_at: chrono::DateTime<chrono::Utc>,
        settings: &ServerSettingsSnapshot,
    ) -> Result<AuthedToken, BbsCgiError> {
        let authed_token = match token {
            Some(token) => self
                .repo
                .get_authed_token(token)
                .await
                .map_err(BbsCgiError::Other)?,
            None => None,
        };
        let Some(authed_token) = authed_token else {
            return self
                .issue_auth_code(ip_addr, user_agent, asn_num, created_at, settings)
                .await;
        };

        if !authed_token.validity {
            if authed_token.authed_at.is_some() {
                return Err(BbsCgiError::RevokedAuthedToken);
            }
            if authed_token.is_activation_expired(Utc::now()) {
                return self
                    .issue_auth_code(ip_addr, user_agent, asn_num, created_at, settings)
                    .await;
            }
            // /auth-code is closed too, so a pending code could not be redeemed anyway.
            if settings.is_enabled(ServerSettingKey::CloseNewAuthentication) {
                return Err(BbsCgiError::NewAuthenticationClosed);
            }
            return Err(BbsCgiError::Unauthenticated {
                auth_code: authed_token.auth_code,
                base_url: self.config.base_url.clone(),
                auth_token: authed_token.token,
            });
        }

        if self.auth_state.is_token_suspended(authed_token.id).await {
            return Err(BbsCgiError::TemporarilySuspended);
        }

        // A one-time code lets the re-auth page identify the token without relying on the
        // IP, which may change on mobile.
        if authed_token.require_reauth {
            return Err(BbsCgiError::ReAuthRequired {
                temp_key: self.auth_state.issue_reauth_code(authed_token.id).await,
                base_url: self.config.base_url.clone(),
            });
        }

        // Check if user registration is required but not linked
        if authed_token.require_user_registration && authed_token.registered_user_id.is_none() {
            if !self
                .user_creation_rate_limit
                .check_and_add(&authed_token.token)
                .await
            {
                return Err(BbsCgiError::TooManyUserCreationAttempt);
            }

            return match create_userreg_temp_url(
                self.auth_state.as_ref(),
                &self.config.base_url,
                &authed_token,
            )
            .await?
            {
                UserRegUrlKind::Registered => Err(BbsCgiError::UserAlreadyRegistered),
                UserRegUrlKind::NotRegistered(user_reg_url) => {
                    Err(BbsCgiError::UserRegistrationRequired { url: user_reg_url })
                }
            };
        }

        Ok(authed_token)
    }
}
