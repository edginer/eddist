use redis::{aio::ConnectionManager, AsyncCommands};
use sqlx::MySql;
use uuid::Uuid;

use crate::{
    repositories::user_repository::UserRepository,
    utils::{redis::user_session_key, TransactionRepository},
};

use super::AppService;

#[derive(Clone)]
pub struct BindTokenToUserService<U: UserRepository> {
    user_repo: U,
    redis_conn: ConnectionManager,
}

impl<U: UserRepository + Clone> BindTokenToUserService<U> {
    pub fn new(user_repo: U, redis_conn: ConnectionManager) -> Self {
        Self {
            user_repo,
            redis_conn,
        }
    }
}

#[async_trait::async_trait]
impl<U: UserRepository + TransactionRepository<MySql> + Clone>
    AppService<BindTokenToUserServiceInput, ()> for BindTokenToUserService<U>
{
    async fn execute(&self, input: BindTokenToUserServiceInput) -> anyhow::Result<()> {
        let mut redis_conn = self.redis_conn.clone();

        // Get user_id from session
        let user_id_str = redis_conn
            .get::<_, Option<String>>(user_session_key(&input.user_sid))
            .await?;

        let Some(user_id_str) = user_id_str else {
            // User session not found, silently return
            return Ok(());
        };

        let user_id = Uuid::parse_str(&user_id_str)?;

        // Check if token is already bound to a user
        if self
            .user_repo
            .is_user_binded_authed_token(input.authed_token_id)
            .await?
        {
            // Already bound, no need to re-bind
            return Ok(());
        }

        // Bind the token to the user
        let tx = self.user_repo.begin().await?;
        let tx = self
            .user_repo
            .bind_user_authed_token(user_id, input.authed_token_id, tx)
            .await?;
        tx.commit().await?;

        log::info!(
            "Auto-bound token {} to user {}",
            input.authed_token_id,
            user_id
        );

        Ok(())
    }
}

pub struct BindTokenToUserServiceInput {
    pub user_sid: String,
    pub authed_token_id: Uuid,
}
