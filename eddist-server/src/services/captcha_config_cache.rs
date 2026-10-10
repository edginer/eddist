use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};

use tokio::sync::RwLock;

use crate::{
    domain::captcha_like::CaptchaProviderConfig,
    repositories::captcha_config_repository::{
        CaptchaConfigRepository, CaptchaConfigRepositoryImpl,
    },
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

/// Get cached captcha configs for the /auth-code endpoint
pub async fn get_cached_captcha_configs_for_auth_code() -> Vec<CaptchaProviderConfig> {
    let cache = get_global_cache().read().await;
    cache
        .configs
        .iter()
        .filter(|c| c.endpoint_usage.matches_auth_code())
        .cloned()
        .collect()
}

/// Get cached captcha configs for the /re-auth endpoint
pub async fn get_cached_captcha_configs_for_reauth() -> Vec<CaptchaProviderConfig> {
    let cache = get_global_cache().read().await;
    cache
        .configs
        .iter()
        .filter(|c| c.endpoint_usage.matches_reauth())
        .cloned()
        .collect()
}

/// On error the previous snapshot is kept; an empty snapshot disables CAPTCHA.
pub async fn refresh_captcha_config_cache(
    repo: &dyn CaptchaConfigRepository,
) -> anyhow::Result<()> {
    refresh_cache(get_global_cache(), repo).await
}

async fn refresh_cache(
    cache: &RwLock<CaptchaConfigCache>,
    repo: &dyn CaptchaConfigRepository,
) -> anyhow::Result<()> {
    let configs = repo.get_active_captcha_configs().await?;
    let mut cache = cache.write().await;

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
pub fn start_captcha_config_refresh_task(pool: sqlx::MySqlPool, refresh_interval: Duration) {
    let repo = CaptchaConfigRepositoryImpl::new(pool);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(refresh_interval);

        loop {
            interval.tick().await;
            if let Err(e) = refresh_captcha_config_cache(&repo).await {
                tracing::error!(
                    error = %format_args!("{e:#}"),
                    "Failed to refresh captcha config cache; keeping the previous snapshot"
                );
            }
        }
    });

    tracing::info!("Started captcha config cache refresh task with interval: {refresh_interval:?}",);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::captcha_like::{CaptchaEndpointUsage, CaptchaWidgetMetadata};

    struct Repository {
        configs: Vec<CaptchaProviderConfig>,
        fail: bool,
    }

    #[async_trait::async_trait]
    impl CaptchaConfigRepository for Repository {
        async fn get_active_captcha_configs(&self) -> anyhow::Result<Vec<CaptchaProviderConfig>> {
            anyhow::ensure!(!self.fail, "database unavailable");
            Ok(self.configs.clone())
        }
    }

    fn valid_config() -> CaptchaProviderConfig {
        CaptchaProviderConfig {
            name: "test".into(),
            provider: "turnstile".into(),
            site_key: "site-key".into(),
            secret: "secret".into(),
            base_url: None,
            widget: CaptchaWidgetMetadata {
                form_field_name: "response".into(),
                script_url: String::new(),
                widget_html: String::new(),
                script_handler: None,
            },
            endpoint_usage: CaptchaEndpointUsage::All,
            capture_fields: Vec::new(),
            verification: None,
        }
    }

    #[tokio::test]
    async fn failed_loads_keep_previous_snapshot_and_valid_empty_load_disables_captcha() {
        let cache = RwLock::new(CaptchaConfigCache::new());
        let valid = Repository {
            configs: vec![valid_config()],
            fail: false,
        };
        refresh_cache(&cache, &valid).await.unwrap();

        let failed = Repository {
            configs: Vec::new(),
            fail: true,
        };
        assert!(refresh_cache(&cache, &failed).await.is_err());
        assert_eq!(cache.read().await.configs.len(), 1);

        let empty = Repository {
            configs: Vec::new(),
            fail: false,
        };
        refresh_cache(&cache, &empty).await.unwrap();
        assert!(cache.read().await.configs.is_empty());
    }

    #[tokio::test]
    async fn initial_load_failure_is_not_a_successful_empty_snapshot() {
        let cache = RwLock::new(CaptchaConfigCache::new());
        let failed = Repository {
            configs: Vec::new(),
            fail: true,
        };
        assert!(refresh_cache(&cache, &failed).await.is_err());
    }
}
