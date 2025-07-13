use chrono::{DateTime, Utc};
use eddist_core::domain::user_restriction_filter::{UserAttributeFilter, UserAttributes};
use serde::{Deserialize, Serialize};
use std::{
    sync::{Arc, LazyLock},
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    error::BbsCgiError, repositories::user_restriction_repository::UserRestrictionRepository,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RestrictionType {
    CreatingResponse,
    CreatingThread,
    AuthCode,
    All,
}

impl RestrictionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RestrictionType::CreatingResponse => "creating_response",
            RestrictionType::CreatingThread => "creating_thread",
            RestrictionType::AuthCode => "auth_code",
            RestrictionType::All => "all",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "creating_response" => Some(RestrictionType::CreatingResponse),
            "creating_thread" => Some(RestrictionType::CreatingThread),
            "auth_code" => Some(RestrictionType::AuthCode),
            "all" => Some(RestrictionType::All),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRestrictionRule {
    pub id: Uuid,
    pub name: String,
    pub filter_expression: String,
    pub restriction_type: RestrictionType,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub description: Option<String>,
}

#[derive(Clone)]
struct CacheEntry {
    rules: Vec<UserRestrictionRule>,
    cached_at: Instant,
}

type RestrictionCache = std::collections::HashMap<RestrictionType, CacheEntry>;

// Global static cache shared across all service instances
static GLOBAL_CACHE: LazyLock<Arc<RwLock<RestrictionCache>>> =
    LazyLock::new(|| Arc::new(RwLock::new(std::collections::HashMap::new())));

/// Service for checking if users are restricted based on rules.
/// This service is READ-ONLY and only checks restrictions - it does not manage rules.
/// Rule management is handled by the admin interface.
#[derive(Clone)]
pub struct UserRestrictionService<T: UserRestrictionRepository> {
    repo: T,
    cache_ttl: Duration,
}

impl<T: UserRestrictionRepository> UserRestrictionService<T> {
    pub fn new(repo: T) -> Self {
        Self {
            repo,
            cache_ttl: Duration::from_secs(300), // 5 minutes cache
        }
    }

    pub fn new_with_cache_ttl(repo: T, cache_ttl: Duration) -> Self {
        Self { repo, cache_ttl }
    }

    /// Check if user attributes match any active restriction rules for a specific type
    pub async fn is_user_restricted(
        &self,
        attributes: &UserAttributes,
        restriction_type: RestrictionType,
    ) -> Result<bool, BbsCgiError> {
        // Check specific restriction type rules
        let specific_rules = self
            .get_active_rules_by_type(restriction_type.clone())
            .await?;
        for rule in specific_rules {
            if let Ok(filter) = UserAttributeFilter::new(rule.filter_expression.clone()) {
                let result = if let Ok(matches) = filter.matches(attributes) {
                    matches
                } else {
                    log::error!(
                        "Failed to parse filter expression: {}",
                        rule.filter_expression
                    );
                    continue; // Skip this rule if parsing failed
                };
            }
        }

        // Check "all" type restrictions (unless the current type is already "all")
        if !matches!(restriction_type, RestrictionType::All) {
            let all_rules = self.get_active_rules_by_type(RestrictionType::All).await?;
            println!("all_rules: {:?}", all_rules);
            for rule in all_rules {
                if let Ok(filter) = UserAttributeFilter::new(rule.filter_expression.clone()) {
                    if filter.matches(attributes).unwrap_or(false) {
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }

    /// Get active restriction rules by type with application-level caching
    async fn get_active_rules_by_type(
        &self,
        restriction_type: RestrictionType,
    ) -> Result<Vec<UserRestrictionRule>, BbsCgiError> {
        // Check cache first
        {
            let cache_read = GLOBAL_CACHE.read().await;
            if let Some(entry) = cache_read.get(&restriction_type) {
                if entry.cached_at.elapsed() < self.cache_ttl {
                    return Ok(entry.rules.clone());
                }
            }
        }

        // Cache miss or expired, fetch from database
        let rules = self
            .repo
            .get_active_user_restriction_rules_by_type(restriction_type.clone())
            .await
            .map_err(BbsCgiError::Other)?;

        // Update cache
        {
            let mut cache_write = GLOBAL_CACHE.write().await;
            cache_write.insert(
                restriction_type,
                CacheEntry {
                    rules: rules.clone(),
                    cached_at: Instant::now(),
                },
            );
        }

        Ok(rules)
    }
}
