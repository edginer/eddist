use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub user_name: String,
    pub enabled: bool,
    pub idp_bindings: Vec<UserIdpBinding>,
    pub authed_token_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct UserIdpBinding {
    pub id: Uuid,
    pub user_id: Uuid,
    pub idp_name: String,
    pub idp_sub: String,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct UserStatusUpdateInput {
    pub enabled: bool,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize, IntoParams)]
pub struct UserSearchQuery {
    pub user_id: Option<Uuid>,
    pub user_name: Option<String>,
    pub authed_token_id: Option<Uuid>,
}
