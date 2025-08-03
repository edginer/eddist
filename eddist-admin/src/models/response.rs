use chrono::{DateTime, Utc};
use eddist_core::domain::{
    client_info::ClientInfo as CoreClientInfo, tinker::Tinker as CoreTinker,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct Res {
    pub id: Uuid,
    pub author_name: Option<String>,
    pub mail: Option<String>,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub author_id: String,
    pub ip_addr: String,
    pub authed_token_id: Uuid,
    pub board_id: Uuid,
    pub thread_id: Uuid,
    pub is_abone: bool,
    pub client_info: ClientInfo,
    pub res_order: i32,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct ClientInfo {
    pub user_agent: String,
    pub asn_num: u32,
    pub ip_addr: String,
    pub tinker: Option<Tinker>,
}

impl From<CoreClientInfo> for ClientInfo {
    fn from(value: CoreClientInfo) -> Self {
        Self {
            user_agent: value.user_agent.to_string(),
            asn_num: value.asn_num,
            ip_addr: value.ip_addr().to_string(),
            tinker: value.tinker.as_deref().cloned().map(Tinker::from),
        }
    }
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct Tinker {
    pub authed_token: String,
    pub wrote_count: u32,
    pub created_thread_count: u32,
    pub level: u32,
    pub last_level_up_at: u64,
    pub last_wrote_at: u64,
    pub last_created_thread_at: Option<u64>,
}

impl From<CoreTinker> for Tinker {
    fn from(value: CoreTinker) -> Self {
        Self {
            authed_token: value.authed_token().to_string(),
            wrote_count: value.wrote_count(),
            created_thread_count: value.created_thread_count(),
            level: value.level(),
            last_level_up_at: value.last_level_up_at(),
            last_wrote_at: value.last_wrote_at(),
            last_created_thread_at: value.last_created_thread_at(),
        }
    }
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct UpdateResInput {
    pub author_name: Option<String>,
    pub mail: Option<String>,
    pub body: Option<String>,
    pub is_abone: Option<bool>,
}
