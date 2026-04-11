use std::collections::HashMap;

use chrono::Utc;
#[cfg(not(feature = "backend-postgres"))]
use sqlx::{Executor, MySqlPool, query, query_as};
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

#[cfg(not(feature = "backend-postgres"))]
#[derive(Debug, Clone)]
pub struct NgWordRepositoryImpl(pub MySqlPool);

#[cfg(not(feature = "backend-postgres"))]
impl NgWordRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self(pool)
    }
}

#[cfg_attr(feature = "backend-postgres", derive(sqlx::FromRow))]
#[derive(Debug)]
pub struct SelectionNgWord {
    pub id: Uuid,
    pub name: String,
    pub word: String,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
    pub board_id: Option<Uuid>,
}

#[cfg(not(feature = "backend-postgres"))]
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

#[cfg(feature = "backend-postgres")]
#[derive(Clone)]
pub struct NgWordRepositoryPgImpl(pub sqlx::PgPool);

#[cfg(feature = "backend-postgres")]
impl NgWordRepositoryPgImpl {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self(pool)
    }
}

#[cfg(feature = "backend-postgres")]
#[async_trait::async_trait]
impl NgWordRepository for NgWordRepositoryPgImpl {
    async fn get_ng_words(&self) -> anyhow::Result<Vec<NgWord>> {
        let selections = sqlx::query_as::<_, SelectionNgWord>(
            r#"
            SELECT
                ng.id,
                name,
                word,
                created_at,
                updated_at,
                board_id
            FROM ng_words AS ng
            LEFT OUTER JOIN boards_ng_words AS bng ON ng.id = bng.ng_word_id
            "#,
        )
        .fetch_all(&self.0)
        .await?;

        let mut ng_words_map = std::collections::HashMap::<_, NgWord>::new();
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
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO ng_words (id, name, word, created_at, updated_at) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id)
        .bind(name)
        .bind(word)
        .bind(now)
        .bind(now)
        .execute(&self.0)
        .await?;

        let selection = sqlx::query_as::<_, SelectionNgWord>(
            r#"
            SELECT
                ng.id, name, word, created_at, updated_at, board_id
            FROM ng_words AS ng
            LEFT OUTER JOIN boards_ng_words AS bng ON ng.id = bng.ng_word_id
            WHERE ng.id = $1
            "#,
        )
        .bind(id)
        .fetch_one(&self.0)
        .await?;

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
        sqlx::query("DELETE FROM ng_words WHERE id = $1")
            .bind(ng_word_id)
            .execute(&self.0)
            .await?;
        Ok(())
    }

    async fn update_ng_word(
        &self,
        id: Uuid,
        name: Option<&str>,
        word: Option<&str>,
        board_ids: Option<Vec<Uuid>>,
    ) -> anyhow::Result<NgWord> {
        let mut sets: Vec<String> = Vec::new();
        let mut idx = 1usize;

        if name.is_some() {
            sets.push(format!("name = ${idx}"));
            idx += 1;
        }
        if word.is_some() {
            sets.push(format!("word = ${idx}"));
            idx += 1;
        }

        if !sets.is_empty() {
            let sql = format!("UPDATE ng_words SET {} WHERE id = ${idx}", sets.join(", "));
            let mut q = sqlx::query(&sql);
            if let Some(v) = name { q = q.bind(v); }
            if let Some(v) = word { q = q.bind(v); }
            q = q.bind(id);
            q.execute(&self.0).await?;
        }

        if let Some(ref bids) = board_ids {
            let mut tx = self.0.begin().await?;

            sqlx::query("DELETE FROM boards_ng_words WHERE ng_word_id = $1")
                .bind(id)
                .execute(&mut *tx)
                .await?;

            for board_id in bids {
                let bnw_id = Uuid::now_v7();
                sqlx::query(
                    "INSERT INTO boards_ng_words (id, board_id, ng_word_id) VALUES ($1, $2, $3)",
                )
                .bind(bnw_id)
                .bind(board_id)
                .bind(id)
                .execute(&mut *tx)
                .await?;
            }

            tx.commit().await?;
        }

        let selections = sqlx::query_as::<_, SelectionNgWord>(
            r#"
            SELECT
                ng.id, name, word, created_at, updated_at, board_id
            FROM ng_words AS ng
            LEFT OUTER JOIN boards_ng_words AS bng ON ng.id = bng.ng_word_id
            WHERE ng.id = $1
            "#,
        )
        .bind(id)
        .fetch_all(&self.0)
        .await?;

        let board_ids_result = selections
            .iter()
            .filter_map(|s| s.board_id)
            .collect::<Vec<_>>();
        let selection = selections.into_iter().next().unwrap();

        Ok(NgWord {
            id: selection.id,
            name: selection.name,
            word: selection.word,
            created_at: selection.created_at,
            updated_at: selection.updated_at,
            board_ids: board_ids_result,
        })
    }
}
