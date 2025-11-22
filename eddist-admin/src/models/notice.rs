use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Notice model for API documentation
#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct Notice {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub content: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub published_at: NaiveDateTime,
    pub author_email: Option<String>,
}

// Conversion from core Notice to admin Notice
impl From<eddist_core::domain::notice::Notice> for Notice {
    fn from(notice: eddist_core::domain::notice::Notice) -> Self {
        Self {
            id: notice.id,
            slug: notice.slug,
            title: notice.title,
            content: notice.content,
            created_at: notice.created_at,
            updated_at: notice.updated_at,
            published_at: notice.published_at,
            author_email: notice.author_email,
        }
    }
}
