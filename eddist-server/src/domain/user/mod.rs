use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod idp;
pub mod user_link_state;
pub mod user_login_state;
pub mod user_reg_state;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub user_name: String,
    pub enabled: bool,
    pub idps: Vec<UserIdp>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserIdp {
    pub idp_id: Uuid,
    pub idp_name: String,
    pub idp_display_name: String,
    pub idp_sub: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
