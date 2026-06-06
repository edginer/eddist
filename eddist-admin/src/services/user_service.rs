use std::sync::Arc;

use uuid::Uuid;

use crate::{
    auth::AdminIdentity,
    models::{User, UserSearchQuery},
    repository::admin_user_repository::AdminUserRepository,
};

#[async_trait::async_trait]
pub trait UserService: Send + Sync {
    async fn search_users(&self, query: UserSearchQuery) -> anyhow::Result<Vec<User>>;
    async fn update_user_status(
        &self,
        actor: &AdminIdentity,
        user_id: Uuid,
        enabled: bool,
    ) -> anyhow::Result<User>;
}

pub struct UserServiceImpl {
    repo: Arc<dyn AdminUserRepository>,
}

impl UserServiceImpl {
    pub fn new(repo: Arc<dyn AdminUserRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait::async_trait]
impl UserService for UserServiceImpl {
    async fn search_users(&self, query: UserSearchQuery) -> anyhow::Result<Vec<User>> {
        self.repo
            .search_users(query.user_id, query.user_name, query.authed_token_id)
            .await
    }

    async fn update_user_status(
        &self,
        _actor: &AdminIdentity,
        user_id: Uuid,
        enabled: bool,
    ) -> anyhow::Result<User> {
        self.repo.update_user_status(user_id, enabled).await?;
        let users = self.repo.search_users(Some(user_id), None, None).await?;
        users.into_iter().next().ok_or_else(|| {
            crate::error::ServiceError::NotFound("User not found after update".into()).into()
        })
    }
}
