use chrono::{NaiveDateTime, Utc};
use eddist_core::domain::notice::Notice;
use sqlx::{query, query_as, MySqlPool};
use uuid::Uuid;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct CreateNoticeInput {
    pub title: String,
    pub content: String,
    pub summary: Option<String>,
    pub published_at: NaiveDateTime,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct UpdateNoticeInput {
    pub title: Option<String>,
    pub content: Option<String>,
    pub summary: Option<String>,
    pub published_at: Option<NaiveDateTime>,
}

#[async_trait::async_trait]
pub trait NoticeRepository: Send + Sync {
    async fn get_all_notices(&self) -> anyhow::Result<Vec<Notice>>;
    async fn get_notices_paginated(&self, page: u32, limit: u32) -> anyhow::Result<Vec<Notice>>;
    async fn get_notice_by_id(&self, id: Uuid) -> anyhow::Result<Option<Notice>>;
    async fn create_notice(
        &self,
        input: CreateNoticeInput,
        author_id: Option<Uuid>,
    ) -> anyhow::Result<Notice>;
    async fn update_notice(&self, id: Uuid, input: UpdateNoticeInput) -> anyhow::Result<Notice>;
    async fn delete_notice(&self, id: Uuid) -> anyhow::Result<()>;
    async fn count_notices(&self) -> anyhow::Result<i64>;
}

#[derive(Clone)]
pub struct NoticeRepositoryImpl(MySqlPool);

impl NoticeRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self(pool)
    }
}

#[async_trait::async_trait]
impl NoticeRepository for NoticeRepositoryImpl {
    async fn get_all_notices(&self) -> anyhow::Result<Vec<Notice>> {
        let notices = query_as!(
            Notice,
            r#"
            SELECT
                id AS "id: Uuid",
                title,
                content,
                summary,
                created_at,
                updated_at,
                published_at,
                author_id AS "author_id: Uuid"
            FROM notices
            ORDER BY published_at DESC
            "#
        )
        .fetch_all(&self.0)
        .await?;

        Ok(notices)
    }

    async fn get_notices_paginated(&self, page: u32, limit: u32) -> anyhow::Result<Vec<Notice>> {
        let offset = page * limit;
        let notices = query_as!(
            Notice,
            r#"
            SELECT
                id AS "id: Uuid",
                title,
                content,
                summary,
                created_at,
                updated_at,
                published_at,
                author_id AS "author_id: Uuid"
            FROM notices
            ORDER BY published_at DESC
            LIMIT ? OFFSET ?
            "#,
            limit,
            offset
        )
        .fetch_all(&self.0)
        .await?;

        Ok(notices)
    }

    async fn get_notice_by_id(&self, id: Uuid) -> anyhow::Result<Option<Notice>> {
        let notice = query_as!(
            Notice,
            r#"
            SELECT
                id AS "id: Uuid",
                title,
                content,
                summary,
                created_at,
                updated_at,
                published_at,
                author_id AS "author_id: Uuid"
            FROM notices
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(&self.0)
        .await?;

        Ok(notice)
    }

    async fn create_notice(
        &self,
        input: CreateNoticeInput,
        author_id: Option<Uuid>,
    ) -> anyhow::Result<Notice> {
        let id = Uuid::new_v4();
        let now = Utc::now().naive_utc();

        query!(
            r#"
            INSERT INTO notices (id, title, content, summary, created_at, updated_at, published_at, author_id)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            id,
            input.title,
            input.content,
            input.summary,
            now,
            now,
            input.published_at,
            author_id
        )
        .execute(&self.0)
        .await?;

        let notice = Notice {
            id,
            title: input.title,
            content: input.content,
            summary: input.summary,
            created_at: now,
            updated_at: now,
            published_at: input.published_at,
            author_id,
        };

        Ok(notice)
    }

    async fn update_notice(&self, id: Uuid, input: UpdateNoticeInput) -> anyhow::Result<Notice> {
        let now = Utc::now().naive_utc();

        // Get the current notice
        let current = self
            .get_notice_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Notice not found"))?;

        // Update only provided fields
        let title = input.title.unwrap_or(current.title);
        let content = input.content.unwrap_or(current.content);
        let summary = input.summary.or(current.summary);
        let published_at = input.published_at.unwrap_or(current.published_at);

        query!(
            r#"
            UPDATE notices
            SET title = ?, content = ?, summary = ?, published_at = ?, updated_at = ?
            WHERE id = ?
            "#,
            title,
            content,
            summary,
            published_at,
            now,
            id
        )
        .execute(&self.0)
        .await?;

        let notice = Notice {
            id,
            title,
            content,
            summary,
            created_at: current.created_at,
            updated_at: now,
            published_at,
            author_id: current.author_id,
        };

        Ok(notice)
    }

    async fn delete_notice(&self, id: Uuid) -> anyhow::Result<()> {
        query!(
            r#"
            DELETE FROM notices
            WHERE id = ?
            "#,
            id
        )
        .execute(&self.0)
        .await?;

        Ok(())
    }

    async fn count_notices(&self) -> anyhow::Result<i64> {
        let count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as count
            FROM notices
            "#
        )
        .fetch_one(&self.0)
        .await?;

        Ok(count)
    }
}
