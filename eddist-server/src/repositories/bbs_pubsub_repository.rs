use eddist_core::domain::pubsub_repository::{
    AuthTokenInitiated, AuthTokenRequested, AuthTokenSucceeded, CreatingRes, PubSubItem,
};
use redis::{AsyncCommands, aio::ConnectionManager};

use super::bbs_repository::CreatingThread;

#[derive(Clone)]
pub struct RedisPubRepository {
    redis_conn: ConnectionManager,
}

impl RedisPubRepository {
    pub fn new(redis_conn: ConnectionManager) -> Self {
        Self { redis_conn }
    }
}

#[async_trait::async_trait]
pub trait PubRepository: Clone + 'static + Send + Sync {
    async fn publish(&self, item: PubSubItem) -> Result<(), anyhow::Error>;
}

#[async_trait::async_trait]
impl PubRepository for RedisPubRepository {
    async fn publish(&self, item: PubSubItem) -> Result<(), anyhow::Error> {
        let mut redis_conn = self.redis_conn.clone();
        let item = serde_json::to_string(&item)?;
        redis_conn
            .publish::<'_, _, _, ()>("bbs:pubsubitem", item.clone())
            .await?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct RedisCreationEventRepository {
    redis_conn: ConnectionManager,
}

impl RedisCreationEventRepository {
    pub fn new(redis_conn: ConnectionManager) -> Self {
        Self { redis_conn }
    }
}

#[async_trait::async_trait]
pub trait CreationEventRepository: Clone + 'static + Send + Sync {
    async fn publish_res_created(&self, event: CreatingRes) -> Result<(), anyhow::Error>;
    async fn publish_thread_created(&self, event: CreatingThread) -> Result<(), anyhow::Error>;
    async fn publish_auth_token_initiated(
        &self,
        event: AuthTokenInitiated,
    ) -> Result<(), anyhow::Error>;
    async fn publish_auth_token_requested(
        &self,
        event: AuthTokenRequested,
    ) -> Result<(), anyhow::Error>;
    async fn publish_auth_token_succeeded(
        &self,
        event: AuthTokenSucceeded,
    ) -> Result<(), anyhow::Error>;
}

#[async_trait::async_trait]
impl CreationEventRepository for RedisCreationEventRepository {
    async fn publish_res_created(&self, event: CreatingRes) -> Result<(), anyhow::Error> {
        let mut redis_conn = self.redis_conn.clone();
        let event = serde_json::to_string(&event)?;
        redis_conn
            .publish::<'_, _, _, ()>("bbs:event:res_created", event)
            .await?;
        Ok(())
    }

    async fn publish_thread_created(&self, event: CreatingThread) -> Result<(), anyhow::Error> {
        let mut redis_conn = self.redis_conn.clone();
        let event = serde_json::to_string(&event)?;
        redis_conn
            .publish::<'_, _, _, ()>("bbs:event:thread_created", event)
            .await?;
        Ok(())
    }

    async fn publish_auth_token_initiated(
        &self,
        event: AuthTokenInitiated,
    ) -> Result<(), anyhow::Error> {
        let mut redis_conn = self.redis_conn.clone();
        let event = serde_json::to_string(&event)?;
        redis_conn
            .publish::<'_, _, _, ()>("bbs:event:auth_token_initiated", event)
            .await?;
        Ok(())
    }

    async fn publish_auth_token_requested(
        &self,
        event: AuthTokenRequested,
    ) -> Result<(), anyhow::Error> {
        let mut redis_conn = self.redis_conn.clone();
        let event = serde_json::to_string(&event)?;
        redis_conn
            .publish::<'_, _, _, ()>("bbs:event:auth_token_requested", event)
            .await?;
        Ok(())
    }

    async fn publish_auth_token_succeeded(
        &self,
        event: AuthTokenSucceeded,
    ) -> Result<(), anyhow::Error> {
        let mut redis_conn = self.redis_conn.clone();
        let event = serde_json::to_string(&event)?;
        redis_conn
            .publish::<'_, _, _, ()>("bbs:event:auth_token_succeeded", event)
            .await?;
        Ok(())
    }
}
