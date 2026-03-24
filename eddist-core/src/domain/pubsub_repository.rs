use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
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
pub struct AuthTokenInitiated {
    pub authed_token_id: Uuid,
    pub origin_ip: String,
    pub user_agent: String,
    pub asn_num: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthTokenRequested {
    pub authed_token_id: Option<Uuid>,
    pub origin_ip: String,
    pub user_agent: String,
    pub asn_num: u32,
    pub auth_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthTokenSucceeded {
    pub authed_token_id: Uuid,
    pub origin_ip: String,
    pub user_agent: String,
    pub asn_num: u32,
    pub authed_at: DateTime<Utc>,
    pub additional_info: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PubSubItem {
    CreatingRes(Box<CreatingRes>),
    Shutdown,
}
