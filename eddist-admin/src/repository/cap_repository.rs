use std::collections::HashMap;

use chrono::Utc;
use uuid::Uuid;

use crate::Cap;

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
pub struct CapRepositoryImpl(pub sqlx::MySqlPool);

impl CapRepositoryImpl {
    pub fn new(pool: sqlx::MySqlPool) -> Self {
        Self(pool)
    }
}

#[derive(Debug)]
pub struct SelectionCap {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
    pub board_id: Option<Uuid>,
}

#[async_trait::async_trait]
impl CapRepository for CapRepositoryImpl {
    async fn get_caps(&self) -> anyhow::Result<Vec<Cap>> {
        let selections = sqlx::query_as!(
            SelectionCap,
            r#"
            SELECT
                cap.id AS "id!: Uuid",
                name AS "name!: String",
                description AS "description!: String",
                created_at AS "created_at!: chrono::DateTime<Utc>",
                updated_at AS "updated_at!: chrono::DateTime<Utc>",
                board_id AS "board_id: Uuid"
            FROM
                caps AS cap
                LEFT OUTER JOIN boards_caps AS bcap
                ON cap.id = bcap.cap_id
            "#,
        )
        .fetch_all(&self.0)
        .await?;

        let mut caps_map = HashMap::<_, Cap>::new();
        for selection in selections {
            caps_map
                .entry(selection.id)
                .and_modify(|x| {
                    if let Some(board_id) = selection.board_id {
                        x.board_ids.push(board_id);
                    }
                })
                .or_insert(Cap {
                    id: selection.id,
                    name: selection.name,
                    description: selection.description,
                    created_at: selection.created_at,
                    updated_at: selection.updated_at,
                    board_ids: if let Some(board_id) = selection.board_id {
                        vec![board_id]
                    } else {
                        Vec::new()
                    },
                });
        }

        Ok(caps_map.into_values().collect())
    }

    async fn create_cap(
        &self,
        name: &str,
        description: &str,
        password_hash: &str,
    ) -> anyhow::Result<Cap> {
        let id = Uuid::now_v7();

        sqlx::query!(
            r#"
            INSERT INTO caps (id, name, description, password_hash, created_at, updated_at)
            VALUES (?, ?, ?, ?, NOW(), NOW())
            "#,
            id,
            name,
            description,
            password_hash
        )
        .execute(&self.0)
        .await?;

        Ok(Cap {
            id,
            name: name.to_string(),
            description: description.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            board_ids: Vec::new(),
        })
    }

    async fn delete_cap(&self, cap_id: Uuid) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
            DELETE FROM boards_caps WHERE cap_id = ?
            "#,
            cap_id
        )
        .execute(&self.0)
        .await?;

        sqlx::query!(
            r#"
            DELETE FROM caps WHERE id = ?
            "#,
            cap_id
        )
        .execute(&self.0)
        .await?;

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
        let mut sets = Vec::new();
        let mut values = Vec::new();

        if let Some(name) = name {
            sets.push("name = ?");
            values.push(name);
        }
        if let Some(description) = description {
            sets.push("description = ?");
            values.push(description);
        }
        if let Some(password_hash) = password_hash {
            sets.push("password_hash = ?");
            values.push(password_hash);
        }

        sets.push("updated_at = NOW()");

        let query = format!(
            r#"
            UPDATE caps
            SET {}
            WHERE id = ?
            "#,
            sets.join(", ")
        );

        let mut query = sqlx::query(&query);
        for value in values {
            query = query.bind(value);
        }
        let query = query.bind(id);
        query.execute(&self.0).await?;

        if let Some(board_ids) = &board_ids {
            let mut tx = self.0.begin().await?;

            let query = sqlx::query!(
                r#"
                DELETE FROM boards_caps WHERE cap_id = ?
                "#,
                id
            );
            query.execute(&mut *tx).await?;

            for board_id in board_ids {
                let bc_id = Uuid::now_v7();
                sqlx::query!(
                    r#"
                    INSERT INTO boards_caps (id, board_id, cap_id)
                    VALUES (?, ?, ?)
                    "#,
                    bc_id,
                    board_id,
                    id
                )
                .execute(&mut *tx)
                .await?;
            }

            tx.commit().await?;
        }

        Ok(Cap {
            id,
            name: name.map_or_else(|| "".to_string(), |x| x.to_string()),
            description: description.map_or_else(|| "".to_string(), |x| x.to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            board_ids: board_ids.unwrap_or_default(),
        })
    }
}
