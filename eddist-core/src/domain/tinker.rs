use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tinker {
    authed_token: String,
    wrote_count: u32,
    created_thread_count: u32,
    level: u32,
    last_level_up_at: u64,
    last_wrote_at: u64,
    last_created_thread_at: Option<u64>,
}

impl Tinker {
    pub fn new(authed_token: String, datetime: DateTime<Utc>) -> Self {
        Self {
            authed_token,
            wrote_count: 0,
            created_thread_count: 0,
            level: 1,
            last_level_up_at: datetime.timestamp() as u64,
            last_wrote_at: 0,
            last_created_thread_at: None,
        }
    }

    pub fn action_on_write(self, datetime: DateTime<Utc>) -> Self {
        let wrote_count = self.wrote_count + 1;
        let timestamp = datetime.timestamp() as u64;
        let level_up = self.last_level_up_at + 23 * 60 * 60 < timestamp;
        let level = if level_up { self.level + 1 } else { self.level };

        Self {
            authed_token: self.authed_token,
            wrote_count,
            created_thread_count: self.created_thread_count,
            level: if level > 20 { 20 } else { level },
            last_level_up_at: if level_up {
                timestamp
            } else {
                self.last_level_up_at
            },
            last_wrote_at: timestamp,
            last_created_thread_at: self.last_created_thread_at,
        }
    }

    pub fn action_on_create_thread(self, datetime: DateTime<Utc>) -> Self {
        let (wrote_count, created_thread_count) =
            (self.wrote_count + 1, self.created_thread_count + 1);
        let timestamp = datetime.timestamp() as u64;
        let level_up = self.last_level_up_at + 23 * 60 * 60 < timestamp;
        let level = if level_up { self.level + 1 } else { self.level };

        Self {
            authed_token: self.authed_token,
            wrote_count,
            created_thread_count,
            level: if level > 20 { 20 } else { level },
            last_level_up_at: if level_up {
                timestamp
            } else {
                self.last_level_up_at
            },
            last_wrote_at: timestamp,
            last_created_thread_at: Some(timestamp),
        }
    }

    pub fn level(&self) -> u32 {
        self.level
    }

    pub fn authed_token(&self) -> &str {
        &self.authed_token
    }

    pub fn wrote_count(&self) -> u32 {
        self.wrote_count
    }

    pub fn created_thread_count(&self) -> u32 {
        self.created_thread_count
    }

    pub fn last_level_up_at(&self) -> u64 {
        self.last_level_up_at
    }

    pub fn last_wrote_at(&self) -> u64 {
        self.last_wrote_at
    }

    pub fn last_created_thread_at(&self) -> Option<u64> {
        self.last_created_thread_at
    }

    pub fn decrement_level(self) -> Self {
        Self {
            level: if self.level > 1 { self.level - 1 } else { 1 },
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_tinker_level_decrement() {
        let tinker = Tinker::new("test_token".to_string(), Utc::now());
        assert_eq!(tinker.level(), 1);

        // Level 1 should not decrement below 1
        let decremented = tinker.decrement_level();
        assert_eq!(decremented.level(), 1);
    }

    #[test]
    fn test_tinker_level_decrement_higher_level() {
        let mut tinker = Tinker::new("test_token".to_string(), Utc::now());
        tinker.level = 5; // Manually set to higher level for testing

        let decremented = tinker.decrement_level();
        assert_eq!(decremented.level(), 4);
    }
}
