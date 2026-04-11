use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
    time::Duration,
};

pub use eddist_core::server_settings::ServerSettingKey;
#[cfg(not(feature = "backend-postgres"))]
use sqlx::MySqlPool;
#[cfg(feature = "backend-postgres")]
use sqlx::PgPool;
use tokio::sync::RwLock;

static GLOBAL_SERVER_SETTINGS_CACHE: OnceLock<Arc<RwLock<HashMap<String, String>>>> =
    OnceLock::new();

fn get_global_cache() -> &'static Arc<RwLock<HashMap<String, String>>> {
    GLOBAL_SERVER_SETTINGS_CACHE.get_or_init(|| Arc::new(RwLock::new(HashMap::new())))
}

pub async fn get_server_setting_bool(key: ServerSettingKey) -> bool {
    get_server_setting(key).await.is_some_and(|v| v == "true")
}

pub async fn get_server_setting(key: ServerSettingKey) -> Option<String> {
    let cache = get_global_cache();
    let map = cache.read().await;
    map.get(key.as_str()).cloned()
}

#[cfg(not(feature = "backend-postgres"))]
pub async fn refresh_server_settings_cache(pool: &MySqlPool) -> anyhow::Result<()> {
    let rows =
        sqlx::query_as::<_, (String, String)>("SELECT setting_key, value FROM server_settings")
            .fetch_all(pool)
            .await?;

    let cache = get_global_cache();
    let mut map = cache.write().await;
    map.clear();
    for (key, value) in rows {
        map.insert(key, value);
    }

    tracing::info!(
        "Server settings cache refreshed with {} settings",
        map.len()
    );
    Ok(())
}

#[cfg(feature = "backend-postgres")]
pub async fn refresh_server_settings_cache(pool: &PgPool) -> anyhow::Result<()> {
    let rows =
        sqlx::query_as::<_, (String, String)>("SELECT setting_key, value FROM server_settings")
            .fetch_all(pool)
            .await?;

    let cache = get_global_cache();
    let mut map = cache.write().await;
    map.clear();
    for (key, value) in rows {
        map.insert(key, value);
    }

    tracing::info!(
        "Server settings cache refreshed with {} settings",
        map.len()
    );
    Ok(())
}

#[cfg(not(feature = "backend-postgres"))]
pub fn start_server_settings_refresh_task(pool: MySqlPool, refresh_interval: Duration) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(refresh_interval);

        loop {
            interval.tick().await;
            if let Err(e) = refresh_server_settings_cache(&pool).await {
                tracing::error!("Failed to refresh server settings cache: {e}");
            }
        }
    });

    tracing::info!(
        "Started server settings cache refresh task with interval: {refresh_interval:?}",
    );
}

#[cfg(feature = "backend-postgres")]
pub fn start_server_settings_refresh_task(pool: PgPool, refresh_interval: Duration) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(refresh_interval);

        loop {
            interval.tick().await;
            if let Err(e) = refresh_server_settings_cache(&pool).await {
                tracing::error!("Failed to refresh server settings cache: {e}");
            }
        }
    });

    tracing::info!(
        "Started server settings cache refresh task with interval: {refresh_interval:?}",
    );
}
