use chrono::Utc;
use eddist_core::domain::terms::Terms;
#[cfg(not(feature = "backend-postgres"))]
use sqlx::{MySqlPool, query, query_as};
use uuid::Uuid;

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

#[cfg(not(feature = "backend-postgres"))]
#[derive(Clone)]
pub struct TermsRepositoryImpl(MySqlPool);

#[cfg(not(feature = "backend-postgres"))]
impl TermsRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self(pool)
    }
}

#[cfg(not(feature = "backend-postgres"))]
#[async_trait::async_trait]
impl TermsRepository for TermsRepositoryImpl {
    async fn get_terms(&self) -> anyhow::Result<Option<Terms>> {
        let terms = query_as!(
            Terms,
            r#"
            SELECT
                id AS "id: Uuid",
                content,
                created_at,
                updated_at,
                updated_by
            FROM terms
            ORDER BY updated_at DESC
            LIMIT 1
            "#
        )
        .fetch_optional(&self.0)
        .await?;

        Ok(terms)
    }

    async fn update_terms(
        &self,
        input: UpdateTermsInput,
        updated_by: Option<String>,
    ) -> anyhow::Result<Terms> {
        let now = Utc::now().naive_utc();

        let current = self
            .get_terms()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Terms not found"))?;

        query!(
            r#"
            UPDATE terms
            SET content = ?, updated_at = ?, updated_by = ?
            WHERE id = ?
            "#,
            input.content,
            now,
            updated_by,
            current.id
        )
        .execute(&self.0)
        .await?;

        let terms = Terms {
            id: current.id,
            content: input.content,
            created_at: current.created_at,
            updated_at: now,
            updated_by,
        };

        Ok(terms)
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
