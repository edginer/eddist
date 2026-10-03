use std::{
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

use eddist_core::domain::user_restriction::{RestrictionTarget, UserRestrictionRule};
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

    fn matching_rule(
        &self,
        ip: &str,
        asn: u32,
        user_agent: &str,
        target: RestrictionTarget,
    ) -> Option<UserRestrictionRule> {
        self.rules
            .iter()
            .find(|rule| rule.target.applies_to(target) && rule.matches(ip, asn, user_agent))
            .cloned()
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
        target: RestrictionTarget,
    ) -> anyhow::Result<Option<UserRestrictionRule>> {
        let global_cache = get_global_cache();
        let cache = global_cache.read().await;
        Ok(cache.matching_rule(ip, asn, user_agent, target))
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
            .is_restricted(&input.ip, input.asn, &input.user_agent, input.target)
            .await?;

        Ok(UserRestrictionCheckOutput { matching_rule })
    }
}

#[derive(Debug, Clone)]
pub struct UserRestrictionCheckInput {
    pub ip: String,
    pub asn: u32,
    pub user_agent: String,
    pub target: RestrictionTarget,
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

#[cfg(test)]
mod tests {
    use super::*;
    use eddist_core::domain::user_restriction::RestrictionRuleType;

    #[test]
    fn cache_skips_rules_for_other_operations_and_expired_rules() {
        let now = chrono::Utc::now();
        let auth = UserRestrictionRule {
            id: uuid::Uuid::new_v4(),
            name: "auth".into(),
            rule_type: RestrictionRuleType::IP,
            rule_value: "192.0.2.1".into(),
            target: RestrictionTarget::Authentication,
            expires_at: None,
            created_at: now,
            updated_at: now,
            created_by_email: "admin@example.com".into(),
        };
        let mut posting = auth.clone();
        posting.id = uuid::Uuid::new_v4();
        posting.target = RestrictionTarget::Posting;
        let mut expired = posting.clone();
        expired.target = RestrictionTarget::Both;
        expired.expires_at = Some(now - chrono::Duration::seconds(1));
        let mut cache = RestrictionCache::new();
        cache.update_rules(vec![expired, auth.clone(), posting.clone()]);
        assert_eq!(
            cache
                .matching_rule("192.0.2.1", 1, "ua", RestrictionTarget::Authentication)
                .unwrap()
                .id,
            auth.id
        );
        assert_eq!(
            cache
                .matching_rule("192.0.2.1", 1, "ua", RestrictionTarget::Posting)
                .unwrap()
                .id,
            posting.id
        );
        cache.update_rules(vec![auth]);
        assert!(
            cache
                .matching_rule("192.0.2.1", 1, "ua", RestrictionTarget::Posting)
                .is_none()
        );
        assert!(
            cache
                .matching_rule("192.0.2.2", 1, "ua", RestrictionTarget::Authentication)
                .is_none()
        );
    }
}
