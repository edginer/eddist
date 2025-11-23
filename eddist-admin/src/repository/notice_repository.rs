use chrono::{NaiveDateTime, Utc};
use eddist_core::domain::notice::Notice;
use sqlx::{query, query_as, MySqlPool};
use uuid::Uuid;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct CreateNoticeInput {
    pub title: String,
    pub slug: String,
    pub content: String,
    pub published_at: NaiveDateTime,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct UpdateNoticeInput {
    pub title: Option<String>,
    pub content: Option<String>,
    pub published_at: Option<NaiveDateTime>,
    /// Optional custom slug. If not provided and title is updated, will be auto-generated from new title.
    pub slug: Option<String>,
}

#[async_trait::async_trait]
pub trait NoticeRepository: Send + Sync {
    async fn get_notices_paginated(&self, page: u32, limit: u32) -> anyhow::Result<Vec<Notice>>;
    async fn get_notice_by_id(&self, id: Uuid) -> anyhow::Result<Option<Notice>>;
    async fn get_notice_by_slug(&self, slug: &str) -> anyhow::Result<Option<Notice>>;
    async fn create_notice(
        &self,
        input: CreateNoticeInput,
        author_email: Option<String>,
    ) -> anyhow::Result<Notice>;
    async fn update_notice(&self, id: Uuid, input: UpdateNoticeInput) -> anyhow::Result<Notice>;
    async fn delete_notice(&self, id: Uuid) -> anyhow::Result<()>;
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
    async fn get_notices_paginated(&self, page: u32, limit: u32) -> anyhow::Result<Vec<Notice>> {
        let offset = page * limit;
        let notices = query_as!(
            Notice,
            r#"
            SELECT
                id AS "id: Uuid",
                slug,
                title,
                content,
                created_at,
                updated_at,
                published_at,
                author_email
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
                slug,
                title,
                content,
                created_at,
                updated_at,
                published_at,
                author_email
            FROM notices
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(&self.0)
        .await?;

        Ok(notice)
    }

    async fn get_notice_by_slug(&self, slug: &str) -> anyhow::Result<Option<Notice>> {
        let notice = query_as!(
            Notice,
            r#"
            SELECT
                id AS "id: Uuid",
                slug,
                title,
                content,
                created_at,
                updated_at,
                published_at,
                author_email
            FROM notices
            WHERE slug = ?
            "#,
            slug
        )
        .fetch_optional(&self.0)
        .await?;

        Ok(notice)
    }

    async fn create_notice(
        &self,
        input: CreateNoticeInput,
        author_email: Option<String>,
    ) -> anyhow::Result<Notice> {
        if input.slug.trim().is_empty() {
            anyhow::bail!("Slug cannot be empty");
        }

        let existing = self.get_notice_by_slug(&input.slug).await?;
        if existing.is_some() {
            anyhow::bail!("Slug already exists");
        }

        let id = Uuid::now_v7();
        let now = Utc::now().naive_utc();

        query!(
            r#"
            INSERT INTO notices (id, slug, title, content, created_at, updated_at, published_at, author_email)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            id,
            input.slug,
            input.title,
            input.content,
            now,
            now,
            input.published_at,
            author_email
        )
        .execute(&self.0)
        .await?;

        let notice = Notice {
            id,
            slug: input.slug,
            title: input.title,
            content: input.content,
            created_at: now,
            updated_at: now,
            published_at: input.published_at,
            author_email,
        };

        Ok(notice)
    }

    async fn update_notice(&self, id: Uuid, input: UpdateNoticeInput) -> anyhow::Result<Notice> {
        let now = Utc::now().naive_utc();

        let current = self
            .get_notice_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Notice not found"))?;

        let title = input.title.clone().unwrap_or_else(|| current.title.clone());
        let content = input.content.unwrap_or(current.content);
        let published_at = input.published_at.unwrap_or(current.published_at);

        let new_slug = if let Some(custom_slug) = input.slug {
            if custom_slug.trim().is_empty() {
                anyhow::bail!("Slug cannot be empty");
            }
            if custom_slug != current.slug {
                let existing = self.get_notice_by_slug(&custom_slug).await?;
                if existing.is_some() {
                    anyhow::bail!("Slug already exists");
                }
            }
            custom_slug
        } else {
            current.slug.clone()
        };

        query!(
            r#"
            UPDATE notices
            SET slug = ?, title = ?, content = ?, published_at = ?, updated_at = ?
            WHERE id = ?
            "#,
            new_slug,
            title,
            content,
            published_at,
            now,
            id
        )
        .execute(&self.0)
        .await?;

        let notice = Notice {
            id,
            slug: new_slug,
            title,
            content,
            created_at: current.created_at,
            updated_at: now,
            published_at,
            author_email: current.author_email,
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
}
