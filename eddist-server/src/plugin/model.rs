use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Plugin {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub script: String,
    pub enabled: bool,
    #[sqlx(json)]
    pub hooks: Vec<PluginHook>,
    #[sqlx(json)]
    pub permissions: PluginPermissions,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PluginHook {
    BeforePostThread,
    AfterPostThread,
    BeforePostResponse,
    AfterPostResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginPermissions {
    #[serde(default)]
    pub allow_http: bool,
    #[serde(default)]
    pub http_whitelist: Vec<HttpWhitelistEntry>,
    #[serde(default)]
    pub allow_storage: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpWhitelistEntry {
    pub url_pattern: String,
    pub methods: Vec<String>,
}

impl HttpWhitelistEntry {
    /// Check if a URL and method are allowed by this whitelist entry
    /// Supports wildcard matching with '*'
    pub fn is_allowed(&self, url: &str, method: &str) -> bool {
        // Check if method is allowed
        let method_allowed = self.methods.iter().any(|m| m.eq_ignore_ascii_case(method));
        if !method_allowed {
            return false;
        }

        // Check if URL matches the pattern
        self.matches_pattern(url)
    }

    fn matches_pattern(&self, url: &str) -> bool {
        // If pattern is exact match (no wildcard)
        if !self.url_pattern.contains('*') {
            return self.url_pattern == url;
        }

        // Split pattern by wildcard
        let parts: Vec<&str> = self.url_pattern.split('*').collect();

        // Pattern must start with first part (if not empty)
        if !parts[0].is_empty() && !url.starts_with(parts[0]) {
            return false;
        }

        // Pattern must end with last part (if not empty)
        if !parts[parts.len() - 1].is_empty() && !url.ends_with(parts[parts.len() - 1]) {
            return false;
        }

        // Check all middle parts exist in order
        let mut search_start = parts[0].len();
        for i in 1..parts.len() - 1 {
            if parts[i].is_empty() {
                continue;
            }
            if let Some(pos) = url[search_start..].find(parts[i]) {
                search_start += pos + parts[i].len();
            } else {
                return false;
            }
        }

        true
    }
}
