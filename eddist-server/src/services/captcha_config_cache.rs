use std::sync::Arc;

use crate::{
    domain::captcha_like::CaptchaProviderConfig,
    repositories::captcha_config_repository::CaptchaConfigRepository,
};

use super::snapshot_cache::SnapshotCache;

#[derive(Debug, Clone)]
pub struct CaptchaConfigCache(SnapshotCache<Vec<CaptchaProviderConfig>>);

impl CaptchaConfigCache {
    /// Unlike a refresh, a failed initial load has no previous snapshot to fall back on.
    pub async fn load(repo: &dyn CaptchaConfigRepository) -> anyhow::Result<Self> {
        let cache = Self::from_configs(Vec::new());
        cache.refresh(repo).await?;
        Ok(cache)
    }

    pub fn from_configs(configs: Vec<CaptchaProviderConfig>) -> Self {
        Self(SnapshotCache::new(configs))
    }

    pub fn for_auth_code(&self) -> Vec<CaptchaProviderConfig> {
        self.filtered(|c| c.endpoint_usage.matches_auth_code())
    }

    pub fn for_reauth(&self) -> Vec<CaptchaProviderConfig> {
        self.filtered(|c| c.endpoint_usage.matches_reauth())
    }

    fn filtered(&self, f: impl Fn(&CaptchaProviderConfig) -> bool) -> Vec<CaptchaProviderConfig> {
        self.snapshot().iter().filter(|c| f(c)).cloned().collect()
    }

    fn snapshot(&self) -> Arc<Vec<CaptchaProviderConfig>> {
        self.0.load()
    }

    /// On error the previous snapshot is kept; an empty snapshot disables CAPTCHA.
    pub async fn refresh(&self, repo: &dyn CaptchaConfigRepository) -> anyhow::Result<()> {
        let configs = repo.get_active_captcha_configs().await?;
        for config in &configs {
            tracing::debug!(
                "Loaded captcha config: provider={}, capture_fields={:?}",
                config.provider,
                config.capture_fields
            );
        }
        tracing::info!(
            "Captcha config cache refreshed with {} configs",
            configs.len()
        );
        self.0.replace(configs);
        Ok(())
    }
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
        let valid = Repository {
            configs: vec![valid_config()],
            fail: false,
        };
        let cache = CaptchaConfigCache::load(&valid).await.unwrap();

        let failed = Repository {
            configs: Vec::new(),
            fail: true,
        };
        assert!(cache.refresh(&failed).await.is_err());
        assert_eq!(cache.snapshot().len(), 1);

        let empty = Repository {
            configs: Vec::new(),
            fail: false,
        };
        cache.refresh(&empty).await.unwrap();
        assert!(cache.snapshot().is_empty());
    }

    #[tokio::test]
    async fn initial_load_failure_is_not_a_successful_empty_snapshot() {
        let failed = Repository {
            configs: Vec::new(),
            fail: true,
        };
        assert!(CaptchaConfigCache::load(&failed).await.is_err());
    }
}
