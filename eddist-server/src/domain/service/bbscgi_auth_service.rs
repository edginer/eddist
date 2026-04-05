use std::{env, sync::OnceLock};

use rand::RngExt;

use chrono::Utc;
use eddist_core::simple_rate_limiter::RateLimiter;
use metrics::counter;
use redis::{AsyncCommands, aio::ConnectionManager};
use tokio::sync::Mutex;

use crate::{
    domain::{
        authed_token::AuthedToken,
        service::bbscgi_user_reg_temp_url_service::{UserRegTempUrlService, UserRegUrlKind},
    },
    error::BbsCgiError,
    repositories::{
        bbs_pubsub_repository::CreationEventRepository,
        bbs_repository::{BbsRepository, CreatingAuthedToken},
    },
};
use eddist_core::{
    domain::pubsub_repository::AuthTokenInitiated,
    redis_keys::{authed_token_suspended_key, reauth_temp_key},
    utils::is_auth_token_pub_enabled,
};

pub static USER_CREATION_RATE_LIMIT: OnceLock<Mutex<RateLimiter>> = OnceLock::new();

#[derive(Clone)]
pub struct BbsCgiAuthService<T: BbsRepository, E: CreationEventRepository> {
    repo: T,
    redis_conn: ConnectionManager,
    event_repo: E,
}

impl<T: BbsRepository, E: CreationEventRepository> BbsCgiAuthService<T, E> {
    pub fn new(repo: T, redis_conn: ConnectionManager, event_repo: E) -> Self {
        Self {
            repo,
            redis_conn,
            event_repo,
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
            tokio::spawn(async move {
                let _ = event_repo
                    .publish_auth_token_initiated(AuthTokenInitiated {
                        authed_token_id,
                        origin_ip,
                        user_agent,
                        asn_num,
                    })
                    .await;
            });
        }
    }

    pub async fn check_validity(
        &self,
        token: Option<&str>,
        ip_addr: String,
        user_agent: String,
        asn_num: i32,
        created_at: chrono::DateTime<chrono::Utc>,
        require_user_registration: bool,
    ) -> Result<AuthedToken, BbsCgiError> {
        let Some(authed_token) = token else {
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

            return Err(BbsCgiError::Unauthenticated {
                auth_code: authed_token.auth_code,
                base_url: env::var("BASE_URL").unwrap(),
                auth_token: authed_token.token,
            });
        };

        let authed_token = self
            .repo
            .get_authed_token(authed_token)
            .await
            .map_err(BbsCgiError::Other)?
            .ok_or_else(|| BbsCgiError::InvalidAuthedToken)?;

        if !authed_token.validity {
            return if authed_token.authed_at.is_some() {
                Err(BbsCgiError::RevokedAuthedToken)
            } else if authed_token.is_activation_expired(Utc::now()) {
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

                return Err(BbsCgiError::Unauthenticated {
                    auth_code: authed_token.auth_code,
                    base_url: env::var("BASE_URL").unwrap(),
                    auth_token: authed_token.token,
                });
            } else {
                Err(BbsCgiError::Unauthenticated {
                    auth_code: authed_token.auth_code,
                    base_url: env::var("BASE_URL").unwrap(),
                    auth_token: authed_token.token,
                })
            };
        }

        // Check temporary suspension flag in Redis
        let suspension_key = authed_token_suspended_key(&authed_token.id.to_string());
        let mut conn = self.redis_conn.clone();
        let is_suspended = conn
            .exists::<_, bool>(&suspension_key)
            .await
            .unwrap_or(false);
        if is_suspended {
            return Err(BbsCgiError::TemporarilySuspended);
        }

        // Check require_reauth flag — generate a one-time temp key so the re-auth page
        // can uniquely identify this token without relying on IP (which may change on mobile).
        if authed_token.require_reauth {
            let temp_key = gen_reauth_temp_key();
            let redis_key = reauth_temp_key(&temp_key);
            conn.set_ex::<_, _, ()>(&redis_key, authed_token.id.to_string(), 3600)
                .await
                .unwrap_or(());
            return Err(BbsCgiError::ReAuthRequired {
                temp_key,
                base_url: env::var("BASE_URL").unwrap(),
            });
        }

        // Check if user registration is required but not linked
        if authed_token.require_user_registration && authed_token.registered_user_id.is_none() {
            let rate_limiter = USER_CREATION_RATE_LIMIT.get_or_init(|| {
                Mutex::new(RateLimiter::new(5, std::time::Duration::from_secs(60 * 60)))
            });
            {
                let mut rate_limiter = rate_limiter.lock().await;
                if !rate_limiter.check_and_add(&authed_token.token) {
                    return Err(BbsCgiError::TooManyUserCreationAttempt);
                }
            }

            let user_reg_url_svc = UserRegTempUrlService::new(self.redis_conn.clone());
            return match user_reg_url_svc
                .create_userreg_temp_url(&authed_token)
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

/// Generates an 8-character Crockford Base32 key (digits + uppercase letters, no I/L/O/U).
/// 32^8 ≈ 1 trillion combinations — sufficient for a 1-hour TTL key.
fn gen_reauth_temp_key() -> String {
    const CHARS: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    let mut rng = rand::rng();
    (0..8)
        .map(|_| CHARS[rng.random_range(0..CHARS.len())] as char)
        .collect()
}
