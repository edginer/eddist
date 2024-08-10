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
        }
    }

    pub fn action_on_write(self, datetime: DateTime<Utc>) -> Self {
        let wrote_count = self.wrote_count + 1;
        let timestamp = datetime.timestamp() as u64;
        let level_up = self.last_level_up_at + 23 * 60 * 60 < timestamp;

        Self {
            authed_token: self.authed_token,
            wrote_count,
            created_thread_count: self.created_thread_count,
            level: if level_up { self.level + 1 } else { self.level },
            last_level_up_at: if level_up {
                timestamp
            } else {
                self.last_level_up_at
            },
            last_wrote_at: timestamp,
        }
    }

    pub fn action_on_create_thread(self, datetime: DateTime<Utc>) -> Self {
        let (wrote_count, created_thread_count) =
            (self.wrote_count + 1, self.created_thread_count + 1);
        let timestamp = datetime.timestamp() as u64;
        let level_up = self.last_level_up_at + 23 * 60 * 60 < timestamp;

        Self {
            authed_token: self.authed_token,
            wrote_count,
            created_thread_count,
            level: if level_up { self.level + 1 } else { self.level },
            last_level_up_at: if level_up {
                timestamp
            } else {
                self.last_level_up_at
            },
            last_wrote_at: timestamp,
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
}
