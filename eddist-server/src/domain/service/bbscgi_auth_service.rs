use std::env;

use chrono::Utc;
use redis::{aio::ConnectionManager, AsyncCommands};

use crate::{
    domain::authed_token::AuthedToken,
    error::BbsCgiError,
    repositories::bbs_repository::{BbsRepository, CreatingAuthedToken},
    utils::{generate_onetime_token, redis::user_link_onetime_key},
};

const ONETIME_TOKEN_LEN: usize = 16;

#[derive(Clone)]
pub struct BbsCgiAuthService<T: BbsRepository> {
    repo: T,
    redis_conn: ConnectionManager,
}

impl<T: BbsRepository> BbsCgiAuthService<T> {
    pub fn new(repo: T, redis_conn: ConnectionManager) -> Self {
        Self { repo, redis_conn }
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
            let authed_token = AuthedToken::new(ip_addr, user_agent, asn_num);
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
                let authed_token = AuthedToken::new(ip_addr, user_agent, asn_num);
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

        // Check if user registration is required but not linked
        if authed_token.require_user_registration && authed_token.registered_user_id.is_none() {
            let ott = generate_onetime_token(ONETIME_TOKEN_LEN);
            let mut redis_conn = self.redis_conn.clone();
            redis_conn
                .set_ex::<_, _, ()>(
                    user_link_onetime_key(&ott),
                    authed_token.id.to_string(),
                    60 * 3, // 3 minutes TTL
                )
                .await
                .map_err(|e| BbsCgiError::Other(e.into()))?;

            return Err(BbsCgiError::UserRegistrationRequired {
                base_url: env::var("BASE_URL").unwrap(),
                ott,
            });
        }

        Ok(authed_token)
    }
}
