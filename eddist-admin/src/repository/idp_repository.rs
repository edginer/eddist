use crate::entity::idp;
use eddist_core::symmetric;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait, QueryOrder};
use uuid::Uuid;

use crate::models::idp::{CreateIdpInput, Idp, UpdateIdpInput};

#[async_trait::async_trait]
pub trait IdpAdminRepository: Send + Sync {
    async fn get_all(&self) -> anyhow::Result<Vec<Idp>>;
    async fn get_by_id(&self, id: Uuid) -> anyhow::Result<Option<Idp>>;
    async fn create(&self, input: CreateIdpInput) -> anyhow::Result<Idp>;
    async fn update(&self, id: Uuid, input: UpdateIdpInput) -> anyhow::Result<Idp>;
    async fn delete(&self, id: Uuid) -> anyhow::Result<()>;
}

#[derive(Clone)]
pub struct IdpAdminRepositoryImpl(DatabaseConnection);

impl IdpAdminRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self(db)
    }
}

fn encrypt_client_secret(plain_secret: &str) -> String {
    symmetric::encrypt(plain_secret)
}

fn into_domain(model: idp::Model) -> Idp {
    Idp {
        id: model.id,
        idp_name: model.idp_name,
        idp_display_name: model.idp_display_name,
        idp_logo_svg: model.idp_logo_svg,
        oidc_config_url: model.oidc_config_url,
        client_id: model.client_id,
        enabled: model.enabled,
    }
}

#[async_trait::async_trait]
impl IdpAdminRepository for IdpAdminRepositoryImpl {
    async fn get_all(&self) -> anyhow::Result<Vec<Idp>> {
        Ok(idp::Entity::find()
            .order_by_asc(idp::Column::IdpName)
            .all(&self.0)
            .await?
            .into_iter()
            .map(into_domain)
            .collect())
    }

    async fn get_by_id(&self, id: Uuid) -> anyhow::Result<Option<Idp>> {
        Ok(idp::Entity::find_by_id(id)
            .one(&self.0)
            .await?
            .map(into_domain))
    }

    async fn create(&self, input: CreateIdpInput) -> anyhow::Result<Idp> {
        let id = Uuid::now_v7();
        let model = idp::ActiveModel {
            id: Set(id),
            idp_name: Set(input.idp_name),
            idp_display_name: Set(input.idp_display_name),
            idp_logo_svg: Set(input.idp_logo_svg),
            oidc_config_url: Set(input.oidc_config_url),
            client_id: Set(input.client_id),
            client_secret: Set(encrypt_client_secret(&input.client_secret)),
            enabled: Set(input.enabled),
        }
        .insert(&self.0)
        .await?;

        Ok(into_domain(model))
    }

    async fn update(&self, id: Uuid, input: UpdateIdpInput) -> anyhow::Result<Idp> {
        let current = self
            .get_by_id(id)
            .await?
            .ok_or_else(|| crate::error::ServiceError::NotFound("IdP not found".into()))?;

        let idp_display_name = input.idp_display_name.unwrap_or(current.idp_display_name);
        let idp_logo_svg = if input.idp_logo_svg.is_some() {
            input.idp_logo_svg
        } else {
            current.idp_logo_svg
        };
        let oidc_config_url = input.oidc_config_url.unwrap_or(current.oidc_config_url);
        let client_id = input.client_id.unwrap_or(current.client_id);
        let enabled = input.enabled.unwrap_or(current.enabled);

        let mut active_model = idp::ActiveModel {
            id: Set(id),
            idp_display_name: Set(idp_display_name),
            idp_logo_svg: Set(idp_logo_svg),
            oidc_config_url: Set(oidc_config_url),
            client_id: Set(client_id),
            enabled: Set(enabled),
            ..Default::default()
        };

        // Only re-encrypt if a new client_secret is provided.
        if let Some(new_secret) = input.client_secret {
            active_model.client_secret = Set(encrypt_client_secret(&new_secret));
        }

        active_model.update(&self.0).await?;

        self.get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("IdP disappeared after update"))
    }

    async fn delete(&self, id: Uuid) -> anyhow::Result<()> {
        idp::Entity::delete_by_id(id).exec(&self.0).await?;
        Ok(())
    }
}
