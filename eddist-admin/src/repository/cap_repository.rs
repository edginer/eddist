use crate::entity::{board_cap, cap};
use crate::error::DbResultExt;
use crate::repository::support::{board_ids_for, replace_board_links};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    IntoActiveValue, QueryFilter, QueryOrder, TransactionTrait,
};
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

#[async_trait::async_trait]
impl CapRepository for CapRepositoryImpl {
    async fn get_caps(&self) -> anyhow::Result<Vec<Cap>> {
        Ok(cap::Entity::find()
            .order_by_asc(cap::Column::Name)
            .find_with_related(board_cap::Entity)
            .all(&self.0)
            .await?
            .into_iter()
            .map(|(model, relations)| {
                let board_ids = relations
                    .into_iter()
                    .map(|relation| relation.board_id)
                    .collect();
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
        let now = crate::db_time::now();
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
        let tx = self.0.begin().await?;

        let updated = cap::ActiveModel {
            id: Set(id),
            updated_at: Set(crate::db_time::now()),
            name: name.map(str::to_string).into_active_value(),
            description: description.map(str::to_string).into_active_value(),
            password_hash: password_hash.map(str::to_string).into_active_value(),
            ..Default::default()
        }
        .update(&tx)
        .await
        .or_not_found("Cap")?;

        if let Some(board_ids) = board_ids {
            replace_board_links::<board_cap::Entity, _, _>(
                &tx,
                board_cap::Column::CapId,
                id,
                board_ids,
                |board_id| board_cap::ActiveModel {
                    id: Set(Uuid::now_v7()),
                    board_id: Set(board_id),
                    cap_id: Set(id),
                },
            )
            .await?;
        }

        let board_ids = board_ids_for::<board_cap::Entity, _>(
            &tx,
            board_cap::Column::CapId,
            id,
            board_cap::Column::BoardId,
        )
        .await?;

        tx.commit().await?;

        Ok(into_domain(updated, board_ids))
    }
}
