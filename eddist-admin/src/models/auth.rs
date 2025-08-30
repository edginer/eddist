use chrono::NaiveDateTime;
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
