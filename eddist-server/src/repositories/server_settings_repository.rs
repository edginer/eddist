use async_trait::async_trait;
use sqlx::MySqlPool;

use crate::services::server_settings_cache::ServerSettingsSnapshot;

#[async_trait]
pub trait ServerSettingsRepository: Send + Sync {
    async fn get_all(&self) -> anyhow::Result<ServerSettingsSnapshot>;
}

#[derive(Clone)]
pub struct ServerSettingsRepositoryImpl {
    pool: MySqlPool,
}

impl ServerSettingsRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ServerSettingsRepository for ServerSettingsRepositoryImpl {
    async fn get_all(&self) -> anyhow::Result<ServerSettingsSnapshot> {
        let rows =
            sqlx::query_as::<_, (String, String)>("SELECT setting_key, value FROM server_settings")
                .fetch_all(&self.pool)
                .await?;
        Ok(ServerSettingsSnapshot::new(rows.into_iter().collect()))
    }
}
