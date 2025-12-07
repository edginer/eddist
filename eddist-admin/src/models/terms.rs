use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Terms model for API documentation
#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct Terms {
    pub id: Uuid,
    pub content: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub updated_by: Option<String>,
}

// Conversion from core Terms to admin Terms
impl From<eddist_core::domain::terms::Terms> for Terms {
    fn from(terms: eddist_core::domain::terms::Terms) -> Self {
        Self {
            id: terms.id,
            content: terms.content,
            created_at: terms.created_at,
            updated_at: terms.updated_at,
            updated_by: terms.updated_by,
        }
    }
}
