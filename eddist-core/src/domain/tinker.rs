use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tinker {
    authed_token: String,
    wrote_count: u32,
    created_thread_count: u32,
    level: u32,
    #[serde(rename = "ilvl", default)]
    internal_level: Option<u32>,
    last_level_up_at: u64,
    last_wrote_at: u64,
    last_created_thread_at: Option<u64>,
}

impl Tinker {
    pub fn from_parts(
        authed_token: String,
        wrote_count: u32,
        created_thread_count: u32,
        level: u32,
        internal_level: u32,
        last_level_up_at: u64,
        last_wrote_at: u64,
        last_created_thread_at: Option<u64>,
    ) -> Self {
        Self {
            authed_token,
            wrote_count,
            created_thread_count,
            level,
            internal_level: Some(internal_level),
            last_level_up_at,
            last_wrote_at,
            last_created_thread_at,
        }
    }

    pub fn new(authed_token: String, datetime: DateTime<Utc>) -> Self {
        Self {
            authed_token,
            wrote_count: 0,
            created_thread_count: 0,
            level: 1,
            internal_level: Some(1),
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
        let internal_level = self
            .internal_level
            .map(|ilvl| (if level_up { ilvl + 1 } else { ilvl }).min(20));

        Self {
            authed_token: self.authed_token,
            wrote_count,
            created_thread_count: self.created_thread_count,
            level: level.min(20),
            internal_level,
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
        let internal_level = self
            .internal_level
            .map(|ilvl| (if level_up { ilvl + 1 } else { ilvl }).min(20));

        Self {
            authed_token: self.authed_token,
            wrote_count,
            created_thread_count,
            level: level.min(20),
            internal_level,
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

    pub fn internal_level(&self) -> u32 {
        self.internal_level.unwrap_or(0)
    }

    /// Patches `internal_level` to `v` only when it is absent (legacy JWT without `ilvl`).
    /// If `internal_level` is already set (including a penalized low value), it is left unchanged.
    pub fn patch_internal_level_if_missing(self, v: u32) -> Self {
        if self.internal_level.is_some() {
            self
        } else {
            Self {
                internal_level: Some(v),
                ..self
            }
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone as _;

    fn legacy_tinker_at_level(level: u32) -> Tinker {
        // Deserialize without the `ilvl` field to simulate a pre-internal_level JWT
        serde_json::from_str(&format!(
            r#"{{"authed_token":"tok","wrote_count":0,"created_thread_count":0,"level":{level},"last_level_up_at":0,"last_wrote_at":0}}"#
        ))
        .unwrap()
    }

    fn far_future() -> DateTime<Utc> {
        // 25 hours past epoch — always triggers a level-up (threshold is 23 h)
        chrono::Utc.timestamp_opt(25 * 3600, 0).unwrap()
    }

    #[test]
    fn patch_if_missing_promotes_legacy_cookie() {
        let t = legacy_tinker_at_level(5);
        assert_eq!(t.internal_level(), 0); // None → 0 before patch
        let patched = t.patch_internal_level_if_missing(5);
        assert_eq!(patched.internal_level(), 5);
    }

    #[test]
    fn patch_if_missing_leaves_explicit_internal_level_unchanged() {
        // internal_level = 2, level = 5 — penalty scenario
        let t = Tinker::from_parts("tok".into(), 0, 0, 5, 2, 0, 0, None);
        let patched = t.patch_internal_level_if_missing(5);
        assert_eq!(patched.internal_level(), 2); // not overwritten
    }

    #[test]
    fn action_on_write_caps_level_and_internal_level_at_20() {
        let t = Tinker::from_parts("tok".into(), 0, 0, 20, 20, 0, 0, None);
        let updated = t.action_on_write(far_future());
        assert_eq!(updated.level(), 20);
        assert_eq!(updated.internal_level(), 20);
    }

    #[test]
    fn action_on_create_thread_caps_level_and_internal_level_at_20() {
        let t = Tinker::from_parts("tok".into(), 0, 0, 20, 20, 0, 0, None);
        let updated = t.action_on_create_thread(far_future());
        assert_eq!(updated.level(), 20);
        assert_eq!(updated.internal_level(), 20);
    }

    #[test]
    fn action_on_write_propagates_none_internal_level() {
        // A legacy Tinker (None internal_level) not yet patched keeps None through action_on_write
        let t = legacy_tinker_at_level(3);
        let updated = t.action_on_write(far_future());
        assert_eq!(updated.internal_level(), 0); // still None → 0
    }
}
