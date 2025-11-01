use eddist_core::domain::notice::{Notice, NoticeListItem};
use sqlx::{query_as, MySqlPool};
use uuid::Uuid;

#[async_trait::async_trait]
pub trait NoticeRepository: Send + Sync + 'static {
    async fn get_latest_notices(&self, limit: u32) -> anyhow::Result<Vec<NoticeListItem>>;
    async fn get_notices_paginated(
        &self,
        page: u32,
        limit: u32,
    ) -> anyhow::Result<Vec<NoticeListItem>>;
    async fn get_notice_by_id(&self, id: Uuid) -> anyhow::Result<Option<Notice>>;
    async fn count_notices(&self) -> anyhow::Result<i64>;
}

#[derive(Debug, Clone)]
pub struct NoticeRepositoryImpl {
    pool: MySqlPool,
}

impl NoticeRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        NoticeRepositoryImpl { pool }
    }
}

#[async_trait::async_trait]
impl NoticeRepository for NoticeRepositoryImpl {
    async fn get_latest_notices(&self, limit: u32) -> anyhow::Result<Vec<NoticeListItem>> {
        let notices = query_as!(
            NoticeListItem,
            r#"
            SELECT
                id AS "id: Uuid",
                title,
                summary,
                published_at
            FROM notices
            WHERE published_at <= NOW()
            ORDER BY published_at DESC
            LIMIT ?
            "#,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(notices)
    }

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
                title,
                summary,
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
            WHERE id = ? AND published_at <= NOW()
            "#,
            id
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
