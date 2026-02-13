use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct AuthedToken {
    pub id: Uuid,
    pub token: String,
    pub origin_ip: String,
    pub reduced_origin_ip: String,
    pub asn_num: i32,
    pub writing_ua: String,
    pub authed_ua: Option<String>,
    pub created_at: NaiveDateTime,
    pub authed_at: Option<NaiveDateTime>,
    pub validity: bool,
    pub last_wrote_at: Option<NaiveDateTime>,
    pub additional_info: Option<serde_json::Value>,
}

#[derive(Debug, Clone, IntoParams, Serialize, Deserialize)]
pub struct ListAuthedTokensQuery {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub origin_ip: Option<String>,
    pub writing_ua: Option<String>,
    pub authed_ua: Option<String>,
    pub asn_num: Option<i32>,
    pub validity: Option<bool>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct PaginatedAuthedTokens {
    pub items: Vec<AuthedToken>,
    pub total: u64,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
}

#[derive(Debug, Clone, ToSchema, IntoParams, Serialize, Deserialize)]
pub struct DeleteAuthedTokenInput {
    pub using_origin_ip: bool,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct NativeSessionRequest {
    pub access_token: String,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct NativeSessionResponse {
    pub session_token: String,
    pub expires_at: DateTime<Utc>,
    pub user_info: NativeUserInfo,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct NativeUserInfo {
    pub sub: String,
    pub email: String,
    pub preferred_username: String,
    pub email_verified: bool,
}
