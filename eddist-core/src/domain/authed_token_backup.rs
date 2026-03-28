use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// S3 key prefix for authed token backups.
pub const AUTHED_TOKENS_S3_PREFIX: &str = "authed_tokens";

/// Snapshot of an authed token stored in S3 for disaster recovery.
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
    /// Seed bytes used to derive the author ID for this token.
    pub author_id_seed: Vec<u8>,
}
