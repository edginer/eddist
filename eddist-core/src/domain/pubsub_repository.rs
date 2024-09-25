use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::client_info::ClientInfo;

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
pub enum PubSubItem {
    CreatingRes(Box<CreatingRes>),
    Shutdown,
}
