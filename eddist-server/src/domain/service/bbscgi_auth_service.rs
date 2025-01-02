use std::env;

use chrono::Utc;

use crate::{
    domain::authed_token::AuthedToken,
    error::BbsCgiError,
    repositories::bbs_repository::{BbsRepository, CreatingAuthedToken},
};

#[derive(Clone)]
pub struct BbsCgiAuthService<T: BbsRepository>(T);

impl<T: BbsRepository> BbsCgiAuthService<T> {
    pub fn new(repo: T) -> Self {
        Self(repo)
    }

    pub async fn check_validity(
        &self,
        token: Option<&str>,
        ip_addr: String,
        user_agent: String,
        created_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<AuthedToken, BbsCgiError> {
        let Some(authed_token) = token else {
            let authed_token = AuthedToken::new(ip_addr, user_agent);
            self.0
                .create_authed_token(CreatingAuthedToken {
                    token: authed_token.token.clone(),
                    writing_ua: authed_token.writing_ua,
                    origin_ip: authed_token.origin_ip,
                    created_at,
                    author_id_seed: authed_token.author_id_seed,
                    auth_code: authed_token.auth_code.clone(),
                    id: authed_token.id,
                })
                .await?;

            return Err(BbsCgiError::Unauthenticated {
                auth_code: authed_token.auth_code,
                base_url: env::var("BASE_URL").unwrap(),
                auth_token: authed_token.token,
            });
        };

        let authed_token = self
            .0
            .get_authed_token(authed_token)
            .await
            .map_err(BbsCgiError::Other)?
            .ok_or_else(|| BbsCgiError::InvalidAuthedToken)?;

        if !authed_token.validity {
            return if authed_token.authed_at.is_some() {
                Err(BbsCgiError::RevokedAuthedToken)
            } else if authed_token.is_activation_expired(Utc::now()) {
                let authed_token = AuthedToken::new(ip_addr, user_agent);
                self.0
                    .create_authed_token(CreatingAuthedToken {
                        token: authed_token.token.clone(),
                        writing_ua: authed_token.writing_ua,
                        origin_ip: authed_token.origin_ip,
                        created_at,
                        author_id_seed: authed_token.author_id_seed,
                        auth_code: authed_token.auth_code.clone(),
                        id: authed_token.id,
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

        Ok(authed_token)
    }
}
