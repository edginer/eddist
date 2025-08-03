use chrono::{DateTime, Utc};
use eddist_core::domain::user_restriction::RestrictionRuleType;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

// NgWord related structs
#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct NgWord {
    pub id: Uuid,
    pub name: String,
    pub word: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub board_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct CreationNgWordInput {
    pub name: String,
    pub word: String,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct UpdateNgWordInput {
    pub name: Option<String>,
    pub word: Option<String>,
    pub board_ids: Option<Vec<Uuid>>,
}

// Cap related structs
#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct Cap {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub board_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct CreationCapInput {
    pub name: String,
    pub description: String,
    pub password: String,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct UpdateCapInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub password: Option<String>,
    pub board_ids: Option<Vec<Uuid>>,
}

// User Restriction related structs
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateRestrictionRuleRequest {
    pub name: String,
    pub rule_type: RestrictionRuleTypeSchema,
    pub rule_value: String,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateRestrictionRuleRequest {
    pub name: Option<String>,
    pub rule_type: Option<RestrictionRuleTypeSchema>,
    pub rule_value: Option<String>,
    pub expires_at: Option<Option<DateTime<Utc>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserRestrictionRuleSchema {
    pub id: String,
    pub name: String,
    pub rule_type: RestrictionRuleTypeSchema,
    pub rule_value: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by_email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum RestrictionRuleTypeSchema {
    Asn,
    IP,
    IPCidr,
    UserAgent,
}

impl From<RestrictionRuleTypeSchema> for RestrictionRuleType {
    fn from(value: RestrictionRuleTypeSchema) -> Self {
        match value {
            RestrictionRuleTypeSchema::Asn => RestrictionRuleType::Asn,
            RestrictionRuleTypeSchema::IP => RestrictionRuleType::IP,
            RestrictionRuleTypeSchema::IPCidr => RestrictionRuleType::IPCidr,
            RestrictionRuleTypeSchema::UserAgent => RestrictionRuleType::UserAgent,
        }
    }
}
