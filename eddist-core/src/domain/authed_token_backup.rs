use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// S3 key prefix for authed token backups.
pub const AUTHED_TOKENS_S3_PREFIX: &str = "authed_tokens";

/// Snapshot of an authed token stored in S3 for disaster recovery.
/// `auth_code` uses `#[serde(default)]` for backwards compatibility with older backup objects
/// that predate the field being included.
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthedTokenBackup {
    pub id: Uuid,
    pub token: String,
    pub origin_ip: String,
    pub reduced_origin_ip: String,
    pub asn_num: i32,
    pub writing_ua: String,
    pub authed_ua: Option<String>,
    #[serde(default)]
    pub auth_code: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub authed_at: Option<chrono::NaiveDateTime>,
    pub last_wrote_at: Option<chrono::NaiveDateTime>,
    pub additional_info: Option<serde_json::Value>,
}
