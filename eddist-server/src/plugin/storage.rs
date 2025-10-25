use anyhow::Result;
use redis::AsyncCommands;
use uuid::Uuid;

pub struct PluginStorage {
    redis: redis::aio::ConnectionManager,
    plugin_id: Uuid,
}

impl PluginStorage {
    pub fn new(redis: redis::aio::ConnectionManager, plugin_id: Uuid) -> Self {
        Self { redis, plugin_id }
    }

    fn key(&self, user_key: &str) -> String {
        format!("plugin:{}:data:{}", self.plugin_id, user_key)
    }

    fn quota_key(&self) -> String {
        format!("plugin:{}:meta:quota", self.plugin_id)
    }

    pub async fn get(&mut self, key: &str) -> Result<Option<String>> {
        let full_key = self.key(key);
        Ok(self.redis.get(&full_key).await?)
    }

    pub async fn set(&mut self, key: &str, value: &str, ttl: Option<i64>) -> Result<bool> {
        let full_key = self.key(key);

        // TODO: Check quota before setting

        if let Some(ttl_secs) = ttl {
            self.redis
                .set_ex::<_, _, ()>(&full_key, value, ttl_secs as u64)
                .await?;
        } else {
            self.redis.set::<_, _, ()>(&full_key, value).await?;
        }

        Ok(true)
    }

    pub async fn delete(&mut self, key: &str) -> Result<bool> {
        let full_key = self.key(key);
        let deleted: bool = self.redis.del(&full_key).await?;

        Ok(deleted)
    }

    pub async fn exists(&mut self, key: &str) -> Result<bool> {
        let full_key = self.key(key);
        Ok(self.redis.exists(&full_key).await?)
    }
}
