use redis::{AsyncCommands, aio::ConnectionManager};

use crate::{domain::user::User, repositories::user_repository::UserRepository};
use eddist_core::redis_keys::user_session_key;

use super::AppService;

#[derive(Clone)]
pub struct UserPageService<U: UserRepository>(U, ConnectionManager);

impl<U: UserRepository> UserPageService<U> {
    pub fn new(user_repo: U, redis_conn: ConnectionManager) -> Self {
        Self(user_repo, redis_conn)
    }
}

#[async_trait::async_trait]
impl<U: UserRepository + Clone>
    AppService<UserPageServiceInput, UserPageServiceOutput, UserPageError> for UserPageService<U>
{
    async fn execute(
        &self,
        input: UserPageServiceInput,
    ) -> Result<UserPageServiceOutput, UserPageError> {
        let mut redis_conn = self.1.clone();
        let Some(user_id) = redis_conn
            .get::<_, Option<String>>(user_session_key(&input.user_sid))
            .await?
        else {
            return Err(UserPageError::SessionNotFound);
        };

        let user_id = user_id.parse::<uuid::Uuid>()?;
        let Some(user) = self.0.get_user_by_id(user_id).await? else {
            return Err(UserPageError::SessionNotFound);
        };

        if !user.enabled {
            return Err(UserPageError::UserDisabled);
        }

        Ok(UserPageServiceOutput { user })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UserPageError {
    #[error("user session not found")]
    SessionNotFound,
    #[error("user is disabled")]
    UserDisabled,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl_from_for_other!(UserPageError: redis::RedisError, uuid::Error);

#[derive(Debug, Clone)]
pub struct UserPageServiceInput {
    pub user_sid: String,
}

#[derive(Debug, Clone)]
pub struct UserPageServiceOutput {
    pub user: User,
}
