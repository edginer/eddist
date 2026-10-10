use std::{collections::HashMap, sync::Arc};

pub use eddist_core::server_settings::ServerSettingKey;

use crate::repositories::server_settings_repository::ServerSettingsRepository;

use super::snapshot_cache::SnapshotCache;

#[derive(Debug, Default, Clone)]
pub struct ServerSettingsSnapshot(HashMap<String, String>);

impl ServerSettingsSnapshot {
    pub fn new(values: HashMap<String, String>) -> Self {
        Self(values)
    }

    pub fn get(&self, key: ServerSettingKey) -> Option<&str> {
        self.0.get(key.as_str()).map(String::as_str)
    }

    pub fn is_enabled(&self, key: ServerSettingKey) -> bool {
        self.get(key) == Some("true")
    }
}

#[derive(Debug, Clone)]
pub struct ServerSettingsCache(SnapshotCache<ServerSettingsSnapshot>);

impl ServerSettingsCache {
    pub async fn load(repo: &dyn ServerSettingsRepository) -> anyhow::Result<Self> {
        Ok(Self::from_snapshot(repo.get_all().await?))
    }

    pub fn from_snapshot(snapshot: ServerSettingsSnapshot) -> Self {
        Self(SnapshotCache::new(snapshot))
    }

    pub fn snapshot(&self) -> Arc<ServerSettingsSnapshot> {
        self.0.load()
    }

    pub async fn refresh(&self, repo: &dyn ServerSettingsRepository) -> anyhow::Result<()> {
        let snapshot = repo.get_all().await?;
        tracing::info!(
            "Server settings cache refreshed with {} settings",
            snapshot.0.len()
        );
        self.0.replace(snapshot);
        Ok(())
    }
}
