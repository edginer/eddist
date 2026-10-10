use eddist_core::domain::user_restriction::{RestrictionTarget, UserRestrictionRule};

use crate::repositories::user_restriction_repository::UserRestrictionRepository;

use super::{AppService, snapshot_cache::SnapshotCache};

#[derive(Debug, Clone)]
pub struct UserRestrictionService {
    cache: UserRestrictionCache,
}

#[derive(Debug, Clone)]
pub struct UserRestrictionCache(SnapshotCache<RestrictionRules>);

#[derive(Debug, Default)]
struct RestrictionRules(Vec<UserRestrictionRule>);

impl RestrictionRules {
    fn matching_rule(
        &self,
        ip: &str,
        asn: u32,
        user_agent: &str,
        target: RestrictionTarget,
    ) -> Option<UserRestrictionRule> {
        self.0
            .iter()
            .find(|rule| rule.target.applies_to(target) && rule.matches(ip, asn, user_agent))
            .cloned()
    }
}

impl UserRestrictionCache {
    pub async fn load(repo: &dyn UserRestrictionRepository) -> anyhow::Result<Self> {
        let cache = Self(SnapshotCache::new(RestrictionRules::default()));
        cache.refresh(repo).await?;
        Ok(cache)
    }

    pub fn empty() -> Self {
        Self(SnapshotCache::new(RestrictionRules::default()))
    }

    pub async fn refresh(&self, repo: &dyn UserRestrictionRepository) -> anyhow::Result<()> {
        let rules = repo.get_all_active_rules().await?;
        tracing::info!(
            "User restriction cache refreshed with {} rules",
            rules.len()
        );
        self.0.replace(RestrictionRules(rules));
        Ok(())
    }

    fn matching_rule(
        &self,
        ip: &str,
        asn: u32,
        user_agent: &str,
        target: RestrictionTarget,
    ) -> Option<UserRestrictionRule> {
        self.0.load().matching_rule(ip, asn, user_agent, target)
    }
}

impl UserRestrictionService {
    pub fn new(cache: UserRestrictionCache) -> Self {
        Self { cache }
    }

    pub async fn is_restricted(
        &self,
        ip: &str,
        asn: u32,
        user_agent: &str,
        target: RestrictionTarget,
    ) -> anyhow::Result<Option<UserRestrictionRule>> {
        Ok(self.cache.matching_rule(ip, asn, user_agent, target))
    }
}

#[async_trait::async_trait]
impl AppService<UserRestrictionCheckInput, UserRestrictionCheckOutput> for UserRestrictionService {
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
        let cache = UserRestrictionCache::empty();
        cache.0.replace(RestrictionRules(vec![
            expired,
            auth.clone(),
            posting.clone(),
        ]));
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
        cache.0.replace(RestrictionRules(vec![auth]));
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
