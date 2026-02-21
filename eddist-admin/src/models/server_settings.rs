use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct ServerSetting {
    pub id: Uuid,
    pub setting_key: String,
    pub value: String,
    pub description: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct UpsertServerSettingInput {
    pub setting_key: String,
    pub value: String,
    pub description: Option<String>,
}
