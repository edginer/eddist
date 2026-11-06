use eddist_entity::idp;
use sea_orm::{DatabaseConnection, EntityTrait};

use crate::domain::user::idp::Idp;

#[async_trait::async_trait]
pub trait IdpRepository: Send + Sync + 'static {
    async fn get_idps(&self) -> anyhow::Result<Vec<Idp>>;
}

#[derive(Debug, Clone)]
pub struct IdpRepositoryImpl {
    db: DatabaseConnection,
}

impl IdpRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl IdpRepository for IdpRepositoryImpl {
    async fn get_idps(&self) -> anyhow::Result<Vec<Idp>> {
        Ok(idp::Entity::find()
            .all(&self.db)
            .await?
            .into_iter()
            .map(|row| Idp {
                id: row.id,
                idp_name: row.idp_name,
                oidc_config_url: row.oidc_config_url,
                idp_display_name: row.idp_display_name,
                idp_logo_svg: row.idp_logo_svg,
                client_id: row.client_id,
                client_secret: row.client_secret,
                enabled: row.enabled,
            })
            .collect())
    }
}
