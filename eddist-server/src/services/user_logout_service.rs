use redis::{aio::ConnectionManager, AsyncCommands};

use crate::utils::redis::user_session_key;

use super::AppService;

#[derive(Clone)]
pub struct UserLogoutService(ConnectionManager);

impl UserLogoutService {
    pub fn new(redis_conn: ConnectionManager) -> Self {
        Self(redis_conn)
    }
}

#[async_trait::async_trait]
impl AppService<UserLogoutServiceInput, ()> for UserLogoutService {
    async fn execute(
        &self,
        UserLogoutServiceInput { user_sid }: UserLogoutServiceInput,
    ) -> anyhow::Result<()> {
        let mut redis_conn = self.0.clone();

        redis_conn
            .del::<_, bool>(user_session_key(&user_sid))
            .await?;

        Ok(())
    }
}

pub struct UserLogoutServiceInput {
    pub user_sid: String,
}
