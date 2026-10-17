use std::sync::Arc;

use crate::stores::user_session_store::UserSessionStore;

use super::AppService;

#[derive(Clone)]
pub struct UserLogoutService(Arc<dyn UserSessionStore>);

impl UserLogoutService {
    pub fn new(sessions: Arc<dyn UserSessionStore>) -> Self {
        Self(sessions)
    }
}

#[async_trait::async_trait]
impl AppService<UserLogoutServiceInput, ()> for UserLogoutService {
    async fn execute(
        &self,
        UserLogoutServiceInput { user_sid }: UserLogoutServiceInput,
    ) -> anyhow::Result<()> {
        self.0.delete(&user_sid).await
    }
}

pub struct UserLogoutServiceInput {
    pub user_sid: String,
}
