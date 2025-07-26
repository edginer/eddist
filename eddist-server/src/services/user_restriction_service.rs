use std::{
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

use eddist_core::domain::user_restriction::UserRestrictionRule;
use tokio::sync::RwLock;

use crate::repositories::user_restriction_repository::UserRestrictionRepository;

use super::AppService;

static GLOBAL_RESTRICTION_CACHE: OnceLock<Arc<RwLock<RestrictionCache>>> = OnceLock::new();

fn get_global_cache() -> &'static Arc<RwLock<RestrictionCache>> {
    GLOBAL_RESTRICTION_CACHE.get_or_init(|| Arc::new(RwLock::new(RestrictionCache::new())))
}

#[derive(Debug, Clone)]
pub struct UserRestrictionService<T: UserRestrictionRepository> {
    repo: T,
}

#[derive(Debug)]
struct RestrictionCache {
    rules: Vec<UserRestrictionRule>,
    last_updated: Instant,
}

impl RestrictionCache {
    fn new() -> Self {
        Self {
            rules: Vec::new(),
            last_updated: Instant::now(),
        }
    }

    fn update_rules(&mut self, rules: Vec<UserRestrictionRule>) {
        self.rules = rules;
        self.last_updated = Instant::now();
    }
}

impl<T: UserRestrictionRepository + Clone> UserRestrictionService<T> {
    pub fn new(repo: T) -> Self {
        Self { repo }
    }

    /// Refresh the cache immediately, typically called by background tasks
    pub async fn refresh_cache(&self) -> anyhow::Result<()> {
        let rules = self.repo.get_all_active_rules().await?;
        let global_cache = get_global_cache();
        let mut cache = global_cache.write().await;
        cache.update_rules(rules);
        tracing::info!(
            "User restriction cache refreshed with {} rules",
            cache.rules.len()
        );
        Ok(())
    }

    pub async fn is_restricted(
        &self,
        ip: &str,
        asn: u32,
        user_agent: &str,
    ) -> anyhow::Result<Option<UserRestrictionRule>> {
        let global_cache = get_global_cache();
        let cache = global_cache.read().await;
        for rule in &cache.rules {
            if rule.matches(ip, asn, user_agent) {
                return Ok(Some(rule.clone()));
            }
        }

        Ok(None)
    }
}

#[async_trait::async_trait]
impl<T: UserRestrictionRepository + Clone>
    AppService<UserRestrictionCheckInput, UserRestrictionCheckOutput>
    for UserRestrictionService<T>
{
    async fn execute(
        &self,
        input: UserRestrictionCheckInput,
    ) -> anyhow::Result<UserRestrictionCheckOutput> {
        let matching_rule = self
            .is_restricted(&input.ip, input.asn, &input.user_agent)
            .await?;

        Ok(UserRestrictionCheckOutput { matching_rule })
    }
}

#[derive(Debug, Clone)]
pub struct UserRestrictionCheckInput {
    pub ip: String,
    pub asn: u32,
    pub user_agent: String,
}

#[derive(Debug, Clone)]
pub struct UserRestrictionCheckOutput {
    pub matching_rule: Option<UserRestrictionRule>,
}

/// Start a background task that periodically refreshes the user restriction cache
pub fn start_cache_refresh_task<T: UserRestrictionRepository + Clone + Send + Sync + 'static>(
    repo: T,
    refresh_interval: Duration,
) {
    tokio::spawn(async move {
        let service = UserRestrictionService::new(repo);
        let mut interval = tokio::time::interval(refresh_interval);

        loop {
            interval.tick().await;
            if let Err(e) = service.refresh_cache().await {
                tracing::error!("Failed to refresh user restriction cache: {e}");
            }
        }
    });

    tracing::info!(
        "Started user restriction cache refresh task with interval: {refresh_interval:?}",
    );
}
