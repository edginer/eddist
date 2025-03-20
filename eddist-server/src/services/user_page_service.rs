use redis::{aio::ConnectionManager, AsyncCommands};

use crate::{domain::user::user::User, repositories::user_repository::UserRepository};

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
        let user_id = redis_conn
            .get::<_, String>(&format!("user:session:{}", input.user_sid))
            .await?;

        let Some(user) = self.0.get_user_by_id(user_id.parse().unwrap()).await? else {
            return Err(anyhow::anyhow!("user not found"));
        };

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
