use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Notice {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub summary: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub published_at: NaiveDateTime,
    pub author_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct NoticeListItem {
    pub id: Uuid,
    pub title: String,
    pub summary: Option<String>,
    pub published_at: NaiveDateTime,
}

impl From<Notice> for NoticeListItem {
    fn from(notice: Notice) -> Self {
        Self {
            id: notice.id,
            title: notice.title,
            summary: notice.summary,
            published_at: notice.published_at,
        }
    }
}
