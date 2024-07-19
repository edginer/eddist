use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Thread {
    pub id: Uuid,
    pub board_id: Uuid,
    pub thread_number: i64,
    pub last_modified_at: DateTime<Utc>,
    pub sage_last_modified_at: DateTime<Utc>,
    pub title: String,
    pub authed_token_id: Uuid,
    pub metadent: String,
    pub response_count: u32,
    pub no_pool: bool,
    pub active: bool,
    pub archived: bool,
}
