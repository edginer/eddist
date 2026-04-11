use eddist_core::domain::terms::Terms;
#[cfg(not(feature = "backend-postgres"))]
use sqlx::{MySqlPool, query_as};
#[cfg(feature = "backend-postgres")]
use sqlx::PgPool;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait TermsRepository: Send + Sync + 'static {
    async fn get_terms(&self) -> anyhow::Result<Option<Terms>>;
}

#[cfg(not(feature = "backend-postgres"))]
#[derive(Debug, Clone)]
pub struct TermsRepositoryImpl {
    pool: MySqlPool,
}

#[cfg(not(feature = "backend-postgres"))]
impl TermsRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        TermsRepositoryImpl { pool }
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
        .fetch_optional(&self.pool)
        .await?;

        Ok(terms)
    }
}

#[cfg(feature = "backend-postgres")]
#[derive(Debug, sqlx::FromRow)]
struct TermsPg {
    pub id: Uuid,
    pub content: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
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

#[cfg(feature = "backend-postgres")]
#[derive(Debug, Clone)]
pub struct TermsRepositoryPgImpl {
    pool: PgPool,
}

#[cfg(feature = "backend-postgres")]
impl TermsRepositoryPgImpl {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[cfg(feature = "backend-postgres")]
#[async_trait::async_trait]
impl TermsRepository for TermsRepositoryPgImpl {
    async fn get_terms(&self) -> anyhow::Result<Option<Terms>> {
        let row = sqlx::query_as::<_, TermsPg>(
            r#"
            SELECT id, content, created_at, updated_at, updated_by
            FROM terms
            ORDER BY updated_at DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Terms::from))
    }
}
