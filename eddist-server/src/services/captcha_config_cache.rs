use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};

use sqlx::MySqlPool;
use tokio::sync::RwLock;

use crate::{
    domain::captcha_like::CaptchaProviderConfig,
    repositories::captcha_config_repository::get_active_captcha_configs,
};

static GLOBAL_CAPTCHA_CONFIG_CACHE: OnceLock<Arc<RwLock<CaptchaConfigCache>>> = OnceLock::new();

fn get_global_cache() -> &'static Arc<RwLock<CaptchaConfigCache>> {
    GLOBAL_CAPTCHA_CONFIG_CACHE.get_or_init(|| Arc::new(RwLock::new(CaptchaConfigCache::new())))
}

#[derive(Debug)]
struct CaptchaConfigCache {
    configs: Vec<CaptchaProviderConfig>,
}

impl CaptchaConfigCache {
    fn new() -> Self {
        Self {
            configs: Vec::new(),
        }
    }

    fn update_configs(&mut self, configs: Vec<CaptchaProviderConfig>) {
        self.configs = configs;
    }
}

/// Get the cached captcha configs
pub async fn get_cached_captcha_configs() -> Vec<CaptchaProviderConfig> {
    let global_cache = get_global_cache();
    let cache = global_cache.read().await;
    cache.configs.clone()
}

/// Refresh the cache with new configs from the database
pub async fn refresh_captcha_config_cache(pool: &MySqlPool) -> anyhow::Result<()> {
    let configs = get_active_captcha_configs(pool).await?;
    let global_cache = get_global_cache();
    let mut cache = global_cache.write().await;

    // Log details about each config for debugging
    for config in &configs {
        tracing::debug!(
            "Loaded captcha config: provider={}, capture_fields={:?}",
            config.provider,
            config.capture_fields
        );
    }

    cache.update_configs(configs);
    tracing::info!(
        "Captcha config cache refreshed with {} configs",
        cache.configs.len()
    );
    Ok(())
}

/// Start a background task that periodically refreshes the captcha config cache
pub fn start_captcha_config_refresh_task(pool: MySqlPool, refresh_interval: Duration) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(refresh_interval);

        loop {
            interval.tick().await;
            if let Err(e) = refresh_captcha_config_cache(&pool).await {
                tracing::error!("Failed to refresh captcha config cache: {e}");
            }
        }
    });

    tracing::info!("Started captcha config cache refresh task with interval: {refresh_interval:?}",);
}
