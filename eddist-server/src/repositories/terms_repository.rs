use eddist_core::domain::terms::Terms;
use eddist_entity::terms;
use sea_orm::{DatabaseConnection, EntityTrait, QueryOrder};

#[async_trait::async_trait]
pub trait TermsRepository: Send + Sync + 'static {
    async fn get_terms(&self) -> anyhow::Result<Option<Terms>>;
}

#[derive(Debug, Clone)]
pub struct TermsRepositoryImpl {
    db: DatabaseConnection,
}

impl TermsRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl TermsRepository for TermsRepositoryImpl {
    async fn get_terms(&self) -> anyhow::Result<Option<Terms>> {
        Ok(terms::Entity::find()
            .order_by_desc(terms::Column::UpdatedAt)
            .one(&self.db)
            .await?
            .map(|row| Terms {
                id: row.id,
                content: row.content,
                created_at: row.created_at,
                updated_at: row.updated_at,
                updated_by: row.updated_by,
            }))
    }
}
