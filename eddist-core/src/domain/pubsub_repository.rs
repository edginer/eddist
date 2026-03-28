use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::client_info::ClientInfo;

pub const CHANNEL_PUBSUB_ITEM: &str = "bbs:pubsubitem";
pub const CHANNEL_AUTH_TOKEN_INITIATED: &str = "bbs:event:auth_token_initiated";
pub const CHANNEL_AUTH_TOKEN_REQUESTED: &str = "bbs:event:auth_token_requested";
pub const CHANNEL_AUTH_TOKEN_SUCCEEDED: &str = "bbs:event:auth_token_succeeded";
pub const CHANNEL_AUTH_TOKEN_REVOKED: &str = "bbs:event:auth_token_revoked";

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
pub struct AuthTokenRevoked {
    pub authed_token_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PubSubItem {
    CreatingRes(Box<CreatingRes>),
    Shutdown,
}
