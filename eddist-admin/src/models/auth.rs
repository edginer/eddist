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
    pub writing_ua: String,
    pub authed_ua: Option<String>,
    pub created_at: NaiveDateTime,
    pub authed_at: Option<NaiveDateTime>,
    pub validity: bool,
    pub last_wrote_at: Option<NaiveDateTime>,
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
