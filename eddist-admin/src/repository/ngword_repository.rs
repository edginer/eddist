use crate::entity::{board_ng_word, ng_word};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, TransactionTrait,
};
use std::collections::HashMap;
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

impl NgWordRepositoryImpl {
    async fn get_ng_word_by_id(&self, id: Uuid) -> anyhow::Result<Option<NgWord>> {
        let Some(model) = ng_word::Entity::find_by_id(id).one(&self.0).await? else {
            return Ok(None);
        };
        let board_ids = board_ng_word::Entity::find()
            .filter(board_ng_word::Column::NgWordId.eq(id))
            .order_by_asc(board_ng_word::Column::BoardId)
            .all(&self.0)
            .await?
            .into_iter()
            .map(|relation| relation.board_id)
            .collect();

        Ok(Some(into_domain(model, board_ids)))
    }
}

#[async_trait::async_trait]
impl NgWordRepository for NgWordRepositoryImpl {
    async fn get_ng_words(&self) -> anyhow::Result<Vec<NgWord>> {
        let models = ng_word::Entity::find()
            .order_by_asc(ng_word::Column::Name)
            .all(&self.0)
            .await?;
        let relations = board_ng_word::Entity::find().all(&self.0).await?;

        let mut board_ids_by_ng_word = HashMap::<Uuid, Vec<Uuid>>::new();
        for relation in relations {
            board_ids_by_ng_word
                .entry(relation.ng_word_id)
                .or_default()
                .push(relation.board_id);
        }

        Ok(models
            .into_iter()
            .map(|model| {
                let board_ids = board_ids_by_ng_word.remove(&model.id).unwrap_or_default();
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
        if ng_word::Entity::find_by_id(id)
            .one(&self.0)
            .await?
            .is_none()
        {
            anyhow::bail!("ng word not found: {id}");
        }

        let now = crate::db_time::now();
        let mut active_model = ng_word::ActiveModel {
            id: Set(id),
            updated_at: Set(now),
            ..Default::default()
        };
        if let Some(name) = name {
            active_model.name = Set(name.to_string());
        }
        if let Some(word) = word {
            active_model.word = Set(word.to_string());
        }

        let tx = self.0.begin().await?;
        active_model.update(&tx).await?;

        if let Some(board_ids) = board_ids {
            board_ng_word::Entity::delete_many()
                .filter(board_ng_word::Column::NgWordId.eq(id))
                .exec(&tx)
                .await?;

            for board_id in board_ids {
                board_ng_word::ActiveModel {
                    id: Set(Uuid::now_v7()),
                    board_id: Set(board_id),
                    ng_word_id: Set(id),
                }
                .insert(&tx)
                .await?;
            }
        }

        tx.commit().await?;

        self.get_ng_word_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("ng word disappeared after update"))
    }
}
