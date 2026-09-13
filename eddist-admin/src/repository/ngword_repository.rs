use crate::entity::{board_ng_word, ng_word};
use crate::error::DbResultExt;
use crate::repository::support::{board_ids_for, replace_board_links};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    IntoActiveValue, QueryFilter, QueryOrder, TransactionTrait,
};
use uuid::Uuid;

use crate::models::NgWord;

#[async_trait::async_trait]
pub trait NgWordRepository: Send + Sync {
    async fn get_ng_words(&self) -> anyhow::Result<Vec<NgWord>>;
    async fn update_ng_word(
        &self,
        id: Uuid,
        name: Option<&str>,
        word: Option<&str>,
        board_ids: Option<Vec<Uuid>>,
    ) -> anyhow::Result<NgWord>;
    async fn create_ng_word(&self, name: &str, word: &str) -> anyhow::Result<NgWord>;
    async fn delete_ng_word(&self, ng_word_id: Uuid) -> anyhow::Result<()>;
}

#[derive(Clone)]
pub struct NgWordRepositoryImpl(DatabaseConnection);

impl NgWordRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self(db)
    }
}

fn into_domain(model: ng_word::Model, board_ids: Vec<Uuid>) -> NgWord {
    NgWord {
        id: model.id,
        name: model.name,
        word: model.word,
        created_at: model.created_at.and_utc(),
        updated_at: model.updated_at.and_utc(),
        board_ids,
    }
}

#[async_trait::async_trait]
impl NgWordRepository for NgWordRepositoryImpl {
    async fn get_ng_words(&self) -> anyhow::Result<Vec<NgWord>> {
        Ok(ng_word::Entity::find()
            .order_by_asc(ng_word::Column::Name)
            .find_with_related(board_ng_word::Entity)
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

    async fn create_ng_word(&self, name: &str, word: &str) -> anyhow::Result<NgWord> {
        let id = Uuid::now_v7();
        let now = crate::db_time::now();
        let model = ng_word::ActiveModel {
            id: Set(id),
            name: Set(name.to_string()),
            word: Set(word.to_string()),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&self.0)
        .await?;

        Ok(into_domain(model, Vec::new()))
    }

    async fn delete_ng_word(&self, ng_word_id: Uuid) -> anyhow::Result<()> {
        let tx = self.0.begin().await?;

        board_ng_word::Entity::delete_many()
            .filter(board_ng_word::Column::NgWordId.eq(ng_word_id))
            .exec(&tx)
            .await?;
        ng_word::Entity::delete_by_id(ng_word_id).exec(&tx).await?;

        tx.commit().await?;
        Ok(())
    }

    async fn update_ng_word(
        &self,
        id: Uuid,
        name: Option<&str>,
        word: Option<&str>,
        board_ids: Option<Vec<Uuid>>,
    ) -> anyhow::Result<NgWord> {
        let tx = self.0.begin().await?;

        let updated = ng_word::ActiveModel {
            id: Set(id),
            updated_at: Set(crate::db_time::now()),
            name: name.map(str::to_string).into_active_value(),
            word: word.map(str::to_string).into_active_value(),
            ..Default::default()
        }
        .update(&tx)
        .await
        .or_not_found("NG word")?;

        if let Some(board_ids) = board_ids {
            replace_board_links::<board_ng_word::Entity, _, _>(
                &tx,
                board_ng_word::Column::NgWordId,
                id,
                board_ids,
                |board_id| board_ng_word::ActiveModel {
                    id: Set(Uuid::now_v7()),
                    board_id: Set(board_id),
                    ng_word_id: Set(id),
                },
            )
            .await?;
        }

        let board_ids = board_ids_for::<board_ng_word::Entity, _>(
            &tx,
            board_ng_word::Column::NgWordId,
            id,
            board_ng_word::Column::BoardId,
        )
        .await?;

        tx.commit().await?;

        Ok(into_domain(updated, board_ids))
    }
}
