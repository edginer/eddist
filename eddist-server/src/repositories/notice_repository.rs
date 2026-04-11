use eddist_core::domain::notice::{Notice, NoticeListItem};
#[cfg(not(feature = "backend-postgres"))]
use sqlx::{MySqlPool, query_as};
#[cfg(feature = "backend-postgres")]
use sqlx::PgPool;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait NoticeRepository: Send + Sync + 'static {
    async fn get_notices_paginated(
        &self,
        page: u32,
        limit: u32,
    ) -> anyhow::Result<Vec<NoticeListItem>>;
    async fn get_notice_by_slug(&self, slug: &str) -> anyhow::Result<Option<Notice>>;
    async fn count_notices(&self) -> anyhow::Result<i64>;
}

#[cfg(not(feature = "backend-postgres"))]
#[derive(Debug, Clone)]
pub struct NoticeRepositoryImpl {
    pool: MySqlPool,
}

#[cfg(not(feature = "backend-postgres"))]
impl NoticeRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        NoticeRepositoryImpl { pool }
    }
}

#[cfg(not(feature = "backend-postgres"))]
#[async_trait::async_trait]
impl NoticeRepository for NoticeRepositoryImpl {
    async fn get_notices_paginated(
        &self,
        page: u32,
        limit: u32,
    ) -> anyhow::Result<Vec<NoticeListItem>> {
        let offset = page * limit;
        let notices = query_as!(
            NoticeListItem,
            r#"
            SELECT
                id AS "id: Uuid",
                slug,
                title,
                published_at
            FROM notices
            WHERE published_at <= NOW()
            ORDER BY published_at DESC
            LIMIT ? OFFSET ?
            "#,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(notices)
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
            WHERE slug = ? AND published_at <= NOW()
            "#,
            slug
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(notice)
    }

    async fn count_notices(&self) -> anyhow::Result<i64> {
        let count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as count
            FROM notices
            WHERE published_at <= NOW()
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }
}

#[cfg(feature = "backend-postgres")]
#[derive(Debug, sqlx::FromRow)]
struct NoticePg {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub content: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub published_at: chrono::DateTime<chrono::Utc>,
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

#[cfg(feature = "backend-postgres")]
#[derive(Debug, sqlx::FromRow)]
struct NoticeListItemPg {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub published_at: chrono::DateTime<chrono::Utc>,
}

#[cfg(feature = "backend-postgres")]
impl From<NoticeListItemPg> for NoticeListItem {
    fn from(r: NoticeListItemPg) -> Self {
        Self {
            id: r.id,
            slug: r.slug,
            title: r.title,
            published_at: r.published_at.naive_utc(),
        }
    }
}

#[cfg(feature = "backend-postgres")]
#[derive(Debug, Clone)]
pub struct NoticeRepositoryPgImpl {
    pool: PgPool,
}

#[cfg(feature = "backend-postgres")]
impl NoticeRepositoryPgImpl {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[cfg(feature = "backend-postgres")]
#[async_trait::async_trait]
impl NoticeRepository for NoticeRepositoryPgImpl {
    async fn get_notices_paginated(
        &self,
        page: u32,
        limit: u32,
    ) -> anyhow::Result<Vec<NoticeListItem>> {
        let offset = (page * limit) as i64;
        let limit = limit as i64;
        let rows = sqlx::query_as::<_, NoticeListItemPg>(
            r#"
            SELECT id, slug, title, published_at
            FROM notices
            WHERE published_at <= NOW()
            ORDER BY published_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(NoticeListItem::from).collect::<Vec<_>>())
    }

    async fn get_notice_by_slug(&self, slug: &str) -> anyhow::Result<Option<Notice>> {
        let row = sqlx::query_as::<_, NoticePg>(
            r#"
            SELECT id, slug, title, content, created_at, updated_at, published_at, author_email
            FROM notices
            WHERE slug = $1 AND published_at <= NOW()
            "#,
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Notice::from))
    }

    async fn count_notices(&self) -> anyhow::Result<i64> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM notices
            WHERE published_at <= NOW()
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }
}
