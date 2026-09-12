use crate::entity::{board_cap, cap};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, TransactionTrait,
};
use std::collections::HashMap;
use uuid::Uuid;

use crate::models::Cap;

#[async_trait::async_trait]
pub trait CapRepository: Send + Sync {
    async fn get_caps(&self) -> anyhow::Result<Vec<Cap>>;
    async fn create_cap(
        &self,
        name: &str,
        description: &str,
        password_hash: &str,
    ) -> anyhow::Result<Cap>;
    async fn delete_cap(&self, cap_id: Uuid) -> anyhow::Result<()>;
    async fn update_cap(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        password_hash: Option<&str>,
        board_ids: Option<Vec<Uuid>>,
    ) -> anyhow::Result<Cap>;
}

#[derive(Clone)]
pub struct CapRepositoryImpl(DatabaseConnection);

impl CapRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self(db)
    }
}

fn into_domain(model: cap::Model, board_ids: Vec<Uuid>) -> Cap {
    Cap {
        id: model.id,
        name: model.name,
        description: model.description,
        created_at: model.created_at.and_utc(),
        updated_at: model.updated_at.and_utc(),
        board_ids,
    }
}

impl CapRepositoryImpl {
    async fn get_cap_by_id(&self, id: Uuid) -> anyhow::Result<Option<Cap>> {
        let Some(model) = cap::Entity::find_by_id(id).one(&self.0).await? else {
            return Ok(None);
        };
        let board_ids = board_cap::Entity::find()
            .filter(board_cap::Column::CapId.eq(id))
            .order_by_asc(board_cap::Column::BoardId)
            .all(&self.0)
            .await?
            .into_iter()
            .map(|relation| relation.board_id)
            .collect();

        Ok(Some(into_domain(model, board_ids)))
    }
}

#[async_trait::async_trait]
impl CapRepository for CapRepositoryImpl {
    async fn get_caps(&self) -> anyhow::Result<Vec<Cap>> {
        let models = cap::Entity::find()
            .order_by_asc(cap::Column::Name)
            .all(&self.0)
            .await?;
        let relations = board_cap::Entity::find().all(&self.0).await?;

        let mut board_ids_by_cap = HashMap::<Uuid, Vec<Uuid>>::new();
        for relation in relations {
            board_ids_by_cap
                .entry(relation.cap_id)
                .or_default()
                .push(relation.board_id);
        }

        Ok(models
            .into_iter()
            .map(|model| {
                let board_ids = board_ids_by_cap.remove(&model.id).unwrap_or_default();
                into_domain(model, board_ids)
            })
            .collect())
    }

    async fn create_cap(
        &self,
        name: &str,
        description: &str,
        password_hash: &str,
    ) -> anyhow::Result<Cap> {
        let id = Uuid::now_v7();
        let now = chrono::Utc::now().naive_utc();
        let model = cap::ActiveModel {
            id: Set(id),
            name: Set(name.to_string()),
            description: Set(description.to_string()),
            password_hash: Set(password_hash.to_string()),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&self.0)
        .await?;

        Ok(into_domain(model, Vec::new()))
    }

    async fn delete_cap(&self, cap_id: Uuid) -> anyhow::Result<()> {
        let tx = self.0.begin().await?;

        board_cap::Entity::delete_many()
            .filter(board_cap::Column::CapId.eq(cap_id))
            .exec(&tx)
            .await?;
        cap::Entity::delete_by_id(cap_id).exec(&tx).await?;

        tx.commit().await?;
        Ok(())
    }

    async fn update_cap(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        password_hash: Option<&str>,
        board_ids: Option<Vec<Uuid>>,
    ) -> anyhow::Result<Cap> {
        if cap::Entity::find_by_id(id).one(&self.0).await?.is_none() {
            anyhow::bail!("cap not found: {id}");
        }

        let now = chrono::Utc::now().naive_utc();
        let mut active_model = cap::ActiveModel {
            id: Set(id),
            updated_at: Set(now),
            ..Default::default()
        };
        if let Some(name) = name {
            active_model.name = Set(name.to_string());
        }
        if let Some(description) = description {
            active_model.description = Set(description.to_string());
        }
        if let Some(password_hash) = password_hash {
            active_model.password_hash = Set(password_hash.to_string());
        }

        let tx = self.0.begin().await?;
        active_model.update(&tx).await?;

        if let Some(board_ids) = board_ids {
            board_cap::Entity::delete_many()
                .filter(board_cap::Column::CapId.eq(id))
                .exec(&tx)
                .await?;

            for board_id in board_ids {
                board_cap::ActiveModel {
                    id: Set(Uuid::now_v7()),
                    board_id: Set(board_id),
                    cap_id: Set(id),
                }
                .insert(&tx)
                .await?;
            }
        }

        tx.commit().await?;

        self.get_cap_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("cap disappeared after update"))
    }
}
