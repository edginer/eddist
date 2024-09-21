use eddist_core::domain::pubsub_repository::PubSubItem;
use redis::{aio::MultiplexedConnection, AsyncCommands};

#[derive(Debug, Clone)]
pub struct RedisPubRepository {
    redis_conn: MultiplexedConnection,
}

impl RedisPubRepository {
    pub fn new(redis_conn: MultiplexedConnection) -> Self {
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
