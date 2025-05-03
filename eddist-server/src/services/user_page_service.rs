use redis::{aio::ConnectionManager, AsyncCommands};

use crate::{
    domain::user::User, repositories::user_repository::UserRepository,
    utils::redis::user_session_key,
};

use super::AppService;

#[derive(Clone)]
pub struct UserPageService<U: UserRepository>(U, ConnectionManager);

impl<U: UserRepository> UserPageService<U> {
    pub fn new(user_repo: U, redis_conn: ConnectionManager) -> Self {
        Self(user_repo, redis_conn)
    }
}

#[async_trait::async_trait]
impl<U: UserRepository + Clone> AppService<UserPageServiceInput, UserPageServiceOutput>
    for UserPageService<U>
{
    async fn execute(&self, input: UserPageServiceInput) -> anyhow::Result<UserPageServiceOutput> {
        let mut redis_conn = self.1.clone();
        let Some(user_id) = redis_conn
            .get::<_, Option<String>>(user_session_key(&input.user_sid))
            .await?
        else {
            return Err(anyhow::anyhow!("user not found"));
        };

        let Some(user) = self.0.get_user_by_id(user_id.parse().unwrap()).await? else {
            return Err(anyhow::anyhow!("user not found"));
        };

        if !user.enabled {
            return Err(anyhow::anyhow!("user not enabled"));
        }

        Ok(UserPageServiceOutput { user })
    }
}

#[derive(Debug, Clone)]
pub struct UserPageServiceInput {
    pub user_sid: String,
}

#[derive(Debug, Clone)]
pub struct UserPageServiceOutput {
    pub user: User,
}
