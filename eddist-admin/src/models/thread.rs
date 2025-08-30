use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct Thread {
    pub id: Uuid,
    pub board_id: Uuid,
    pub thread_number: u64,
    pub last_modified: DateTime<Utc>,
    pub sage_last_modified: DateTime<Utc>,
    pub title: String,
    pub authed_token_id: Uuid,
    pub metadent: String,
    pub response_count: u32,
    pub no_pool: bool,
    pub archived: bool,
    pub active: bool,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct ThreadCompactionInput {
    pub target_count: u32,
}
