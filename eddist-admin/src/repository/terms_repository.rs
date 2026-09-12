use crate::entity::terms;
use chrono::Utc;
use eddist_core::domain::terms::Terms;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait, QueryOrder};

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
        let now = Utc::now().naive_utc();

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
