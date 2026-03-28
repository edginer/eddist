use eddist_core::domain::pubsub_repository::{
    AuthTokenInitiated, AuthTokenRequested, AuthTokenRevoked, AuthTokenSucceeded, CreatingRes,
    PubSubItem,
};
use redis::{AsyncCommands, aio::ConnectionManager};
use serde::Serialize;

use super::bbs_repository::CreatingThread;
use eddist_core::redis_keys::{
    CHANNEL_AUTH_TOKEN_INITIATED, CHANNEL_AUTH_TOKEN_REQUESTED, CHANNEL_AUTH_TOKEN_REVOKED,
    CHANNEL_AUTH_TOKEN_SUCCEEDED, CHANNEL_RES_CREATED, CHANNEL_THREAD_CREATED,
};

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

    async fn publish_to_channel<T: Serialize>(
        &self,
        channel: &str,
        event: T,
    ) -> Result<(), anyhow::Error> {
        let mut redis_conn = self.redis_conn.clone();
        let payload = serde_json::to_string(&event)?;
        redis_conn.publish::<'_, _, _, ()>(channel, payload).await?;
        Ok(())
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
    async fn publish_auth_token_revoked(
        &self,
        event: AuthTokenRevoked,
    ) -> Result<(), anyhow::Error>;
}

#[async_trait::async_trait]
impl CreationEventRepository for RedisCreationEventRepository {
    async fn publish_res_created(&self, event: CreatingRes) -> Result<(), anyhow::Error> {
        self.publish_to_channel(CHANNEL_RES_CREATED, event).await
    }

    async fn publish_thread_created(&self, event: CreatingThread) -> Result<(), anyhow::Error> {
        self.publish_to_channel(CHANNEL_THREAD_CREATED, event).await
    }

    async fn publish_auth_token_initiated(
        &self,
        event: AuthTokenInitiated,
    ) -> Result<(), anyhow::Error> {
        self.publish_to_channel(CHANNEL_AUTH_TOKEN_INITIATED, event)
            .await
    }

    async fn publish_auth_token_requested(
        &self,
        event: AuthTokenRequested,
    ) -> Result<(), anyhow::Error> {
        self.publish_to_channel(CHANNEL_AUTH_TOKEN_REQUESTED, event)
            .await
    }

    async fn publish_auth_token_succeeded(
        &self,
        event: AuthTokenSucceeded,
    ) -> Result<(), anyhow::Error> {
        self.publish_to_channel(CHANNEL_AUTH_TOKEN_SUCCEEDED, event)
            .await
    }

    async fn publish_auth_token_revoked(
        &self,
        event: AuthTokenRevoked,
    ) -> Result<(), anyhow::Error> {
        self.publish_to_channel(CHANNEL_AUTH_TOKEN_REVOKED, event)
            .await
    }
}
