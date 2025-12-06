use chrono::Utc;
use eddist_core::domain::terms::Terms;
use sqlx::{query, query_as, MySqlPool};
use uuid::Uuid;

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
pub struct TermsRepositoryImpl(MySqlPool);

impl TermsRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self(pool)
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
