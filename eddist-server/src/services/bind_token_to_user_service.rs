use std::sync::Arc;

use sqlx::MySql;
use uuid::Uuid;

use crate::{
    repositories::user_repository::UserRepository, stores::user_session_store::UserSessionStore,
    utils::TransactionRepository,
};

use super::AppService;

#[derive(Clone)]
pub struct BindTokenToUserService<U: UserRepository> {
    user_repo: U,
    sessions: Arc<dyn UserSessionStore>,
}

impl<U: UserRepository + Clone> BindTokenToUserService<U> {
    pub fn new(user_repo: U, sessions: Arc<dyn UserSessionStore>) -> Self {
        Self {
            user_repo,
            sessions,
        }
    }
}

#[async_trait::async_trait]
impl<U: UserRepository + TransactionRepository<MySql> + Clone>
    AppService<BindTokenToUserServiceInput, ()> for BindTokenToUserService<U>
{
    async fn execute(&self, input: BindTokenToUserServiceInput) -> anyhow::Result<()> {
        let Some(user_id_str) = self.sessions.get_user_id(&input.user_sid).await? else {
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
