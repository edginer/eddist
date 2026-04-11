use crate::entity::terms;
use eddist_core::domain::terms::Terms;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait, QueryOrder};

#[cfg(feature = "backend-postgres")]
#[derive(Debug, sqlx::FromRow)]
struct TermsPg {
    pub id: Uuid,
    pub content: String,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
    pub updated_by: Option<String>,
}

#[cfg(feature = "backend-postgres")]
impl From<TermsPg> for Terms {
    fn from(r: TermsPg) -> Self {
        Self {
            id: r.id,
            content: r.content,
            created_at: r.created_at.naive_utc(),
            updated_at: r.updated_at.naive_utc(),
            updated_by: r.updated_by,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, utoipa::ToSchema)]
pub struct UpdateTermsInput {
    pub content: String,
}

#[async_trait::async_trait]
pub trait TermsRepository: Send + Sync {
    async fn get_terms(&self) -> anyhow::Result<Option<Terms>>;
    async fn update_terms(
        &self,
        input: UpdateTermsInput,
        updated_by: Option<String>,
    ) -> anyhow::Result<Terms>;
}

#[derive(Clone)]
pub struct TermsRepositoryImpl(DatabaseConnection);

impl TermsRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self(db)
    }
}

fn into_domain(model: terms::Model) -> Terms {
    Terms {
        id: model.id,
        content: model.content,
        created_at: model.created_at,
        updated_at: model.updated_at,
        updated_by: model.updated_by,
    }
}

#[async_trait::async_trait]
impl TermsRepository for TermsRepositoryImpl {
    async fn get_terms(&self) -> anyhow::Result<Option<Terms>> {
        Ok(terms::Entity::find()
            .order_by_desc(terms::Column::UpdatedAt)
            .one(&self.0)
            .await?
            .map(into_domain))
    }

    async fn update_terms(
        &self,
        input: UpdateTermsInput,
        updated_by: Option<String>,
    ) -> anyhow::Result<Terms> {
        let now = crate::db_time::now();

        let current = self
            .get_terms()
            .await?
            .ok_or_else(|| crate::error::ServiceError::NotFound("Terms not found".into()))?;

        let updated = terms::ActiveModel {
            id: Set(current.id),
            content: Set(input.content),
            updated_at: Set(now),
            updated_by: Set(updated_by),
            ..Default::default()
        }
        .update(&self.0)
        .await?;

        Ok(into_domain(updated))
    }
}

#[cfg(feature = "backend-postgres")]
#[derive(Clone)]
pub struct TermsRepositoryPgImpl(sqlx::PgPool);

#[cfg(feature = "backend-postgres")]
impl TermsRepositoryPgImpl {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self(pool)
    }

    async fn get_terms_pg(&self) -> anyhow::Result<Option<Terms>> {
        let row = sqlx::query_as::<_, TermsPg>(
            r#"
            SELECT id, content, created_at, updated_at, updated_by
            FROM terms
            ORDER BY updated_at DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.0)
        .await?;
        Ok(row.map(Terms::from))
    }
}

#[cfg(feature = "backend-postgres")]
#[async_trait::async_trait]
impl TermsRepository for TermsRepositoryPgImpl {
    async fn get_terms(&self) -> anyhow::Result<Option<Terms>> {
        self.get_terms_pg().await
    }

    async fn update_terms(
        &self,
        input: UpdateTermsInput,
        updated_by: Option<String>,
    ) -> anyhow::Result<Terms> {
        let now = Utc::now();

        let current = self
            .get_terms_pg()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Terms not found"))?;

        sqlx::query(
            r#"
            UPDATE terms
            SET content = $1, updated_at = $2, updated_by = $3
            WHERE id = $4
            "#,
        )
        .bind(&input.content)
        .bind(now)
        .bind(&updated_by)
        .bind(current.id)
        .execute(&self.0)
        .await?;

        Ok(Terms {
            id: current.id,
            content: input.content,
            created_at: current.created_at,
            updated_at: now.naive_utc(),
            updated_by,
        })
    }
}
