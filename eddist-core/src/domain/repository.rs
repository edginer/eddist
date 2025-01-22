use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{client_info::ClientInfo, metadent::MetadentType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatingRes {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub body: String,
    pub name: String,
    pub mail: String,
    pub author_ch5id: String,
    pub authed_token_id: Uuid,
    pub ip_addr: String,
    pub thread_id: Uuid,
    pub board_id: Uuid,
    pub client_info: ClientInfo,
    pub res_order: i32,
    pub is_sage: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatingThread {
    pub thread_id: Uuid,
    pub response_id: Uuid,
    pub title: String,
    pub unix_time: u64,
    pub body: String,
    pub name: String,
    pub mail: String,
    pub created_at: DateTime<Utc>,
    pub author_ch5id: String,
    pub authed_token_id: Uuid,
    pub ip_addr: String,
    pub board_id: Uuid,
    pub metadent: MetadentType,
    pub client_info: ClientInfo,
}
