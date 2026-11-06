use async_trait::async_trait;
use eddist_entity::server_settings;
use sea_orm::{DatabaseConnection, EntityTrait, QuerySelect};

use crate::services::server_settings_cache::ServerSettingsSnapshot;

#[async_trait]
pub trait ServerSettingsRepository: Send + Sync {
    async fn get_all(&self) -> anyhow::Result<ServerSettingsSnapshot>;
}

#[derive(Clone)]
pub struct ServerSettingsRepositoryImpl {
    db: DatabaseConnection,
}

impl ServerSettingsRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl ServerSettingsRepository for ServerSettingsRepositoryImpl {
    async fn get_all(&self) -> anyhow::Result<ServerSettingsSnapshot> {
        let rows = server_settings::Entity::find()
            .select_only()
            .columns([
                server_settings::Column::SettingKey,
                server_settings::Column::Value,
            ])
            .into_tuple::<(String, String)>()
            .all(&self.db)
            .await?;
        Ok(ServerSettingsSnapshot::new(rows.into_iter().collect()))
    }
}
