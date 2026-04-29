use eddist_core::{
    domain::pubsub_repository::{
        AuthTokenInitiated, AuthTokenRequested, AuthTokenSucceeded, CreatingRes, PubSubItem,
    },
    proto::{
        encode_auth_token_initiated, encode_auth_token_requested, encode_auth_token_succeeded,
        encode_creating_res, encode_creating_thread,
    },
};
use redis::{AsyncCommands, aio::ConnectionManager};

use super::bbs_repository::CreatingThread;
use eddist_core::redis_keys::{
    CHANNEL_AUTH_TOKEN_INITIATED, CHANNEL_AUTH_TOKEN_REQUESTED, CHANNEL_AUTH_TOKEN_SUCCEEDED,
    CHANNEL_RES_CREATED, CHANNEL_THREAD_CREATED,
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

    async fn publish_bytes(&self, channel: &str, payload: Vec<u8>) -> Result<(), anyhow::Error> {
        let mut redis_conn = self.redis_conn.clone();
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
}

#[async_trait::async_trait]
impl CreationEventRepository for RedisCreationEventRepository {
    async fn publish_res_created(&self, event: CreatingRes) -> Result<(), anyhow::Error> {
        self.publish_bytes(CHANNEL_RES_CREATED, encode_creating_res(&event))
            .await
    }

    async fn publish_thread_created(&self, event: CreatingThread) -> Result<(), anyhow::Error> {
        self.publish_bytes(CHANNEL_THREAD_CREATED, encode_creating_thread(&event))
            .await
    }

    async fn publish_auth_token_initiated(
        &self,
        event: AuthTokenInitiated,
    ) -> Result<(), anyhow::Error> {
        self.publish_bytes(
            CHANNEL_AUTH_TOKEN_INITIATED,
            encode_auth_token_initiated(&event),
        )
        .await
    }

    async fn publish_auth_token_requested(
        &self,
        event: AuthTokenRequested,
    ) -> Result<(), anyhow::Error> {
        self.publish_bytes(
            CHANNEL_AUTH_TOKEN_REQUESTED,
            encode_auth_token_requested(&event),
        )
        .await
    }

    async fn publish_auth_token_succeeded(
        &self,
        event: AuthTokenSucceeded,
    ) -> Result<(), anyhow::Error> {
        self.publish_bytes(
            CHANNEL_AUTH_TOKEN_SUCCEEDED,
            encode_auth_token_succeeded(&event),
        )
        .await
    }
}
