use eddist_core::domain::notice::{Notice, NoticeListItem};
use eddist_entity::notice;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect,
    sea_query::{Expr, ExprTrait},
};
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

#[derive(Debug, Clone)]
pub struct NoticeRepositoryImpl {
    db: DatabaseConnection,
}

impl NoticeRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[derive(FromQueryResult)]
struct NoticeListRow {
    id: Uuid,
    slug: String,
    title: String,
    published_at: chrono::NaiveDateTime,
}

fn published_notices() -> sea_orm::Select<notice::Entity> {
    notice::Entity::find().filter(
        Expr::col((notice::Entity, notice::Column::PublishedAt)).lte(Expr::current_timestamp()),
    )
}

fn listed_notices() -> sea_orm::Select<notice::Entity> {
    published_notices().filter(notice::Column::HideFromList.eq(false))
}

#[async_trait::async_trait]
impl NoticeRepository for NoticeRepositoryImpl {
    async fn get_notices_paginated(
        &self,
        page: u32,
        limit: u32,
    ) -> anyhow::Result<Vec<NoticeListItem>> {
        let offset = page * limit;
        let notices = listed_notices()
            .select_only()
            .columns([
                notice::Column::Id,
                notice::Column::Slug,
                notice::Column::Title,
                notice::Column::PublishedAt,
            ])
            .order_by_desc(notice::Column::PublishedAt)
            .limit(u64::from(limit))
            .offset(u64::from(offset))
            .into_model::<NoticeListRow>()
            .all(&self.db)
            .await?;

        Ok(notices
            .into_iter()
            .map(|row| NoticeListItem {
                id: row.id,
                slug: row.slug,
                title: row.title,
                published_at: row.published_at,
            })
            .collect())
    }

    async fn get_notice_by_slug(&self, slug: &str) -> anyhow::Result<Option<Notice>> {
        Ok(published_notices()
            .filter(notice::Column::Slug.eq(slug))
            .one(&self.db)
            .await?
            .map(|row| Notice {
                id: row.id,
                slug: row.slug,
                title: row.title,
                content: row.content,
                created_at: row.created_at,
                updated_at: row.updated_at,
                published_at: row.published_at,
                author_email: row.author_email,
                hide_from_list: row.hide_from_list,
            }))
    }

    async fn count_notices(&self) -> anyhow::Result<i64> {
        Ok(i64::try_from(listed_notices().count(&self.db).await?)?)
    }
}
