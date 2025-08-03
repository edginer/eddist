use std::collections::HashMap;

use chrono::Utc;
use sqlx::{query, query_as, Executor, MySqlPool};
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

#[derive(Debug, Clone)]
pub struct NgWordRepositoryImpl(pub MySqlPool);

impl NgWordRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self(pool)
    }
}

#[derive(Debug)]
pub struct SelectionNgWord {
    pub id: Uuid,
    pub name: String,
    pub word: String,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
    pub board_id: Option<Uuid>,
}

#[async_trait::async_trait]
impl NgWordRepository for NgWordRepositoryImpl {
    async fn get_ng_words(&self) -> anyhow::Result<Vec<NgWord>> {
        let selections = query_as!(
            SelectionNgWord,
            r#"
            SELECT
                ng.id AS "id!: Uuid",
                name AS "name!: String",
                word AS "word!: String",
                created_at AS "created_at!: chrono::DateTime<Utc>",
                updated_at AS "updated_at!: chrono::DateTime<Utc>",
                board_id AS "board_id: Uuid"
            FROM
                ng_words AS ng
                LEFT OUTER JOIN boards_ng_words AS bng
                ON ng.id = bng.ng_word_id
            "#,
        )
        .fetch_all(&self.0)
        .await?;

        let mut ng_words_map = HashMap::<_, NgWord>::new();
        for selection in selections {
            ng_words_map
                .entry(selection.id)
                .and_modify(|x| {
                    if let Some(board_id) = selection.board_id {
                        x.board_ids.push(board_id);
                    }
                })
                .or_insert(NgWord {
                    id: selection.id,
                    name: selection.name,
                    word: selection.word,
                    created_at: selection.created_at,
                    updated_at: selection.updated_at,
                    board_ids: if let Some(board_id) = selection.board_id {
                        vec![board_id]
                    } else {
                        Vec::new()
                    },
                });
        }

        Ok(ng_words_map.into_values().collect())
    }

    async fn create_ng_word(&self, name: &str, word: &str) -> anyhow::Result<NgWord> {
        let id = Uuid::now_v7();

        let query = query!(
            r#"
            INSERT INTO
                ng_words (id, name, word, created_at, updated_at)
            VALUES
                (?, ?, ?, NOW(), NOW())
        "#,
            id,
            name,
            word
        );
        self.0.execute(query).await?;

        let query = query_as!(
            SelectionNgWord,
            r#"
            SELECT
                ng.id AS "id!: Uuid",
                name AS "name!: String",
                word AS "word!: String",
                created_at AS "created_at!: chrono::DateTime<Utc>",
                updated_at AS "updated_at!: chrono::DateTime<Utc>",
                board_id AS "board_id: Uuid"
            FROM
                ng_words AS ng
                LEFT OUTER JOIN boards_ng_words AS bng
                ON ng.id = bng.ng_word_id
            WHERE
                ng.id = ?
            "#,
            id,
        );

        let selection = query.fetch_one(&self.0).await?;

        Ok(NgWord {
            id: selection.id,
            name: selection.name,
            word: selection.word,
            created_at: selection.created_at,
            updated_at: selection.updated_at,
            board_ids: if let Some(board_id) = selection.board_id {
                vec![board_id]
            } else {
                Vec::new()
            },
        })
    }

    async fn delete_ng_word(&self, ng_word_id: Uuid) -> anyhow::Result<()> {
        let query = query!(
            r#"
            DELETE FROM
                ng_words
            WHERE
                id = ?
        "#,
            ng_word_id
        );

        self.0.execute(query).await?;

        Ok(())
    }

    async fn update_ng_word(
        &self,
        id: Uuid,
        name: Option<&str>,
        word: Option<&str>,
        board_ids: Option<Vec<Uuid>>,
    ) -> anyhow::Result<NgWord> {
        let mut sets = Vec::new();
        let mut values = Vec::new();

        if let Some(name) = name {
            sets.push("name = ?");
            values.push(name);
        }
        if let Some(word) = word {
            sets.push("word = ?");
            values.push(word);
        }

        let query = format!(
            r#"
            UPDATE
                ng_words
            SET
                {}
            WHERE
                id = ?
            "#,
            sets.join(", ")
        );

        let mut query = sqlx::query(&query);
        for v in values {
            query = query.bind(v);
        }
        let query = query.bind(id);
        query.execute(&self.0).await?;

        if let Some(board_ids) = board_ids {
            let mut tx = self.0.begin().await?;

            let query = query!(
                r#"
                DELETE FROM
                    boards_ng_words
                WHERE
                    ng_word_id = ?
            "#,
                id
            );
            tx.execute(query).await?;

            for board_id in board_ids {
                let bnw_id = Uuid::now_v7();
                let query = query!(
                    r#"
                    INSERT INTO
                        boards_ng_words (id, board_id, ng_word_id)
                    VALUES
                        (?, ?, ?)
                "#,
                    bnw_id,
                    board_id,
                    id
                );
                tx.execute(query).await?;
            }

            tx.commit().await?;
        }

        let query = query_as!(
            SelectionNgWord,
            r#"
            SELECT
                ng.id AS "id!: Uuid",
                name AS "name!: String",
                word AS "word!: String",
                created_at AS "created_at!: chrono::DateTime<Utc>",
                updated_at AS "updated_at!: chrono::DateTime<Utc>",
                board_id AS "board_id: Uuid"
            FROM
                ng_words AS ng
                LEFT OUTER JOIN boards_ng_words AS bng
                ON ng.id = bng.ng_word_id
            WHERE
                ng.id = ?
            "#,
            id,
        );

        let selections = query.fetch_all(&self.0).await?;
        let board_ids = selections
            .iter()
            .filter_map(|selection| selection.board_id)
            .collect::<Vec<_>>();
        let selection = selections.into_iter().next().unwrap();

        Ok(NgWord {
            id: selection.id,
            name: selection.name,
            word: selection.word,
            created_at: selection.created_at,
            updated_at: selection.updated_at,
            board_ids,
        })
    }
}
