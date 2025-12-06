use eddist_core::domain::terms::Terms;
use sqlx::{query_as, MySqlPool};
use uuid::Uuid;

#[async_trait::async_trait]
pub trait TermsRepository: Send + Sync + 'static {
    async fn get_terms(&self) -> anyhow::Result<Option<Terms>>;
}

#[derive(Debug, Clone)]
pub struct TermsRepositoryImpl {
    pool: MySqlPool,
}

impl TermsRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        TermsRepositoryImpl { pool }
    }
}

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
