use chrono::{NaiveDateTime, Utc};
use eddist_core::domain::notice::Notice;
#[cfg(not(feature = "backend-postgres"))]
use sqlx::{MySqlPool, query, query_as};
use uuid::Uuid;

#[cfg(feature = "backend-postgres")]
#[derive(Debug, sqlx::FromRow)]
struct NoticePg {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub content: String,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
    pub published_at: chrono::DateTime<Utc>,
    pub author_email: Option<String>,
}

#[cfg(feature = "backend-postgres")]
impl From<NoticePg> for Notice {
    fn from(r: NoticePg) -> Self {
        Self {
            id: r.id,
            slug: r.slug,
            title: r.title,
            content: r.content,
            created_at: r.created_at.naive_utc(),
            updated_at: r.updated_at.naive_utc(),
            published_at: r.published_at.naive_utc(),
            author_email: r.author_email,
        }
    }
}

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

#[cfg(not(feature = "backend-postgres"))]
#[derive(Clone)]
pub struct NoticeRepositoryImpl(MySqlPool);

#[cfg(not(feature = "backend-postgres"))]
impl NoticeRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self(pool)
    }
}

#[cfg(not(feature = "backend-postgres"))]
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

#[cfg(feature = "backend-postgres")]
#[derive(Clone)]
pub struct NoticeRepositoryPgImpl(sqlx::PgPool);

#[cfg(feature = "backend-postgres")]
impl NoticeRepositoryPgImpl {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self(pool)
    }

    async fn get_by_id_pg(&self, id: Uuid) -> anyhow::Result<Option<Notice>> {
        let row = sqlx::query_as::<_, NoticePg>(
            r#"
            SELECT id, slug, title, content, created_at, updated_at, published_at, author_email
            FROM notices
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.0)
        .await?;
        Ok(row.map(Notice::from))
    }

    async fn get_by_slug_pg(&self, slug: &str) -> anyhow::Result<Option<Notice>> {
        let row = sqlx::query_as::<_, NoticePg>(
            r#"
            SELECT id, slug, title, content, created_at, updated_at, published_at, author_email
            FROM notices
            WHERE slug = $1
            "#,
        )
        .bind(slug)
        .fetch_optional(&self.0)
        .await?;
        Ok(row.map(Notice::from))
    }
}

#[cfg(feature = "backend-postgres")]
#[async_trait::async_trait]
impl NoticeRepository for NoticeRepositoryPgImpl {
    async fn get_notices_paginated(&self, page: u32, limit: u32) -> anyhow::Result<Vec<Notice>> {
        let offset = (page * limit) as i64;
        let rows = sqlx::query_as::<_, NoticePg>(
            r#"
            SELECT id, slug, title, content, created_at, updated_at, published_at, author_email
            FROM notices
            ORDER BY published_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit as i64)
        .bind(offset)
        .fetch_all(&self.0)
        .await?;
        Ok(rows.into_iter().map(Notice::from).collect())
    }

    async fn get_notice_by_id(&self, id: Uuid) -> anyhow::Result<Option<Notice>> {
        self.get_by_id_pg(id).await
    }

    async fn get_notice_by_slug(&self, slug: &str) -> anyhow::Result<Option<Notice>> {
        self.get_by_slug_pg(slug).await
    }

    async fn create_notice(
        &self,
        input: CreateNoticeInput,
        author_email: Option<String>,
    ) -> anyhow::Result<Notice> {
        if input.slug.trim().is_empty() {
            anyhow::bail!("Slug cannot be empty");
        }

        let existing = self.get_by_slug_pg(&input.slug).await?;
        if existing.is_some() {
            anyhow::bail!("Slug already exists");
        }

        let id = Uuid::now_v7();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO notices (id, slug, title, content, created_at, updated_at, published_at, author_email)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(id)
        .bind(&input.slug)
        .bind(&input.title)
        .bind(&input.content)
        .bind(now)
        .bind(now)
        .bind(chrono::DateTime::<Utc>::from_naive_utc_and_offset(input.published_at, Utc))
        .bind(&author_email)
        .execute(&self.0)
        .await?;

        Ok(Notice {
            id,
            slug: input.slug,
            title: input.title,
            content: input.content,
            created_at: now.naive_utc(),
            updated_at: now.naive_utc(),
            published_at: input.published_at,
            author_email,
        })
    }

    async fn update_notice(&self, id: Uuid, input: UpdateNoticeInput) -> anyhow::Result<Notice> {
        let now = Utc::now();

        let current = self
            .get_by_id_pg(id)
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
                let existing = self.get_by_slug_pg(&custom_slug).await?;
                if existing.is_some() {
                    anyhow::bail!("Slug already exists");
                }
            }
            custom_slug
        } else {
            current.slug.clone()
        };

        sqlx::query(
            r#"
            UPDATE notices
            SET slug = $1, title = $2, content = $3, published_at = $4, updated_at = $5
            WHERE id = $6
            "#,
        )
        .bind(&new_slug)
        .bind(&title)
        .bind(&content)
        .bind(chrono::DateTime::<Utc>::from_naive_utc_and_offset(published_at, Utc))
        .bind(now)
        .bind(id)
        .execute(&self.0)
        .await?;

        Ok(Notice {
            id,
            slug: new_slug,
            title,
            content,
            created_at: current.created_at,
            updated_at: now.naive_utc(),
            published_at,
            author_email: current.author_email,
        })
    }

    async fn delete_notice(&self, id: Uuid) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM notices WHERE id = $1")
            .bind(id)
            .execute(&self.0)
            .await?;
        Ok(())
    }
}
