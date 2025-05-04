use redis::{aio::ConnectionManager, AsyncCommands};
use serde::Serialize;

use crate::{
    repositories::{idp_repository::IdpRepository, user_repository::UserRepository},
    utils::redis::user_session_key,
};

use super::AppService;

#[derive(Clone)]
pub struct UserLoginPageService<U: UserRepository, I: IdpRepository>(U, I, ConnectionManager);

impl<U: UserRepository, I: IdpRepository> UserLoginPageService<U, I> {
    pub fn new(user_repo: U, idp_repo: I, redis_conn: ConnectionManager) -> Self {
        Self(user_repo, idp_repo, redis_conn)
    }
}

#[async_trait::async_trait]
impl<U: UserRepository + Clone, I: IdpRepository + Clone>
    AppService<UserLoginPageServiceInput, UserLoginPageServiceOutput>
    for UserLoginPageService<U, I>
{
    async fn execute(
        &self,
        input: UserLoginPageServiceInput,
    ) -> anyhow::Result<UserLoginPageServiceOutput> {
        let mut redis_conn = self.2.clone();

        if let Some(user_sid) = input.user_sid {
            if redis_conn
                .exists::<_, bool>(user_session_key(&user_sid))
                .await?
            {
                return Ok(UserLoginPageServiceOutput::LoggedIn);
            }
        }

        let idps = self.1.get_idps().await?;
        let available_idps = idps
            .into_iter()
            .filter(|idp| idp.enabled)
            .map(|idp| AvailableIdp {
                idp_name: idp.idp_name,
                idp_display_name: idp.idp_display_name,
                idp_logo_svg: idp.idp_logo_svg,
            })
            .collect::<Vec<_>>();

        if available_idps.is_empty() {
            return Err(anyhow::anyhow!("User login is not available"));
        }

        Ok(UserLoginPageServiceOutput::NotLoggedIn { available_idps })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AvailableIdp {
    pub idp_name: String,
    pub idp_display_name: String,
    pub idp_logo_svg: Option<String>,
}

pub struct UserLoginPageServiceInput {
    pub user_sid: Option<String>,
}

pub enum UserLoginPageServiceOutput {
    LoggedIn,
    NotLoggedIn { available_idps: Vec<AvailableIdp> },
}
