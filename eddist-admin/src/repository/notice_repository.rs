use crate::entity::notice;
use chrono::NaiveDateTime;
use eddist_core::domain::notice::Notice;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect,
};
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
    pub hide_from_list: bool,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct UpdateNoticeInput {
    pub title: Option<String>,
    pub content: Option<String>,
    pub published_at: Option<NaiveDateTime>,
    /// Optional custom slug. If not provided and title is updated, will be auto-generated from new title.
    pub slug: Option<String>,
    pub hide_from_list: Option<bool>,
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
pub struct NoticeRepositoryImpl(DatabaseConnection);

impl NoticeRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self(db)
    }
}

fn into_domain(model: notice::Model) -> Notice {
    Notice {
        id: model.id,
        slug: model.slug,
        title: model.title,
        content: model.content,
        created_at: model.created_at,
        updated_at: model.updated_at,
        published_at: model.published_at,
        author_email: model.author_email,
        hide_from_list: model.hide_from_list,
    }
}

#[async_trait::async_trait]
impl NoticeRepository for NoticeRepositoryImpl {
    async fn get_notices_paginated(&self, page: u32, limit: u32) -> anyhow::Result<Vec<Notice>> {
        let offset = u64::from(page) * u64::from(limit);
        let notices = notice::Entity::find()
            .order_by_desc(notice::Column::PublishedAt)
            .limit(u64::from(limit))
            .offset(offset)
            .all(&self.0)
            .await?;

        Ok(notices.into_iter().map(into_domain).collect())
    }

    async fn get_notice_by_id(&self, id: Uuid) -> anyhow::Result<Option<Notice>> {
        Ok(notice::Entity::find_by_id(id)
            .one(&self.0)
            .await?
            .map(into_domain))
    }

    async fn get_notice_by_slug(&self, slug: &str) -> anyhow::Result<Option<Notice>> {
        Ok(notice::Entity::find()
            .filter(notice::Column::Slug.eq(slug))
            .one(&self.0)
            .await?
            .map(into_domain))
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
        let now = crate::db_time::now();

        let model = notice::ActiveModel {
            id: Set(id),
            slug: Set(input.slug),
            title: Set(input.title),
            content: Set(input.content),
            created_at: Set(now),
            updated_at: Set(now),
            published_at: Set(crate::db_time::truncate_to_millis(input.published_at)),
            author_email: Set(author_email),
            hide_from_list: Set(input.hide_from_list),
        }
        .insert(&self.0)
        .await?;

        Ok(into_domain(model))
    }

    async fn update_notice(&self, id: Uuid, input: UpdateNoticeInput) -> anyhow::Result<Notice> {
        let now = crate::db_time::now();

        let current = self
            .get_notice_by_id(id)
            .await?
            .ok_or_else(|| crate::error::ServiceError::NotFound("Notice not found".into()))?;

        let title = input.title.clone().unwrap_or_else(|| current.title.clone());
        let content = input.content.unwrap_or(current.content);
        let published_at =
            crate::db_time::truncate_to_millis(input.published_at.unwrap_or(current.published_at));
        let hide_from_list = input.hide_from_list.unwrap_or(current.hide_from_list);

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

        let updated = notice::ActiveModel {
            id: Set(id),
            slug: Set(new_slug),
            title: Set(title),
            content: Set(content),
            published_at: Set(published_at),
            updated_at: Set(now),
            hide_from_list: Set(hide_from_list),
            ..Default::default()
        }
        .update(&self.0)
        .await?;

        Ok(into_domain(updated))
    }

    async fn delete_notice(&self, id: Uuid) -> anyhow::Result<()> {
        notice::Entity::delete_by_id(id).exec(&self.0).await?;

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
        .bind(chrono::DateTime::<Utc>::from_naive_utc_and_offset(
            published_at,
            Utc,
        ))
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
