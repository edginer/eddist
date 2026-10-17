use std::sync::Arc;

use crate::{
    domain::user::User, repositories::user_repository::UserRepository,
    stores::user_session_store::UserSessionStore,
};

use super::AppService;

#[derive(Clone)]
pub struct UserPageService<U: UserRepository>(U, Arc<dyn UserSessionStore>);

impl<U: UserRepository> UserPageService<U> {
    pub fn new(user_repo: U, sessions: Arc<dyn UserSessionStore>) -> Self {
        Self(user_repo, sessions)
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
        let Some(user_id) = self.1.get_user_id(&input.user_sid).await? else {
            return Err(UserPageError::SessionNotFound);
        };

        let Ok(user_id) = user_id.parse() else {
            return Err(UserPageError::SessionNotFound);
        };
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

#[derive(Debug, Clone)]
pub struct UserPageServiceInput {
    pub user_sid: String,
}

#[derive(Debug, Clone)]
pub struct UserPageServiceOutput {
    pub user: User,
}
