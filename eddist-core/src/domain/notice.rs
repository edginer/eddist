use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoticeListItem {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub published_at: NaiveDateTime,
}

impl From<Notice> for NoticeListItem {
    fn from(notice: Notice) -> Self {
        Self {
            id: notice.id,
            slug: notice.slug,
            title: notice.title,
            published_at: notice.published_at,
        }
    }
}
