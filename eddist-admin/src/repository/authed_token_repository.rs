use sqlx::{query, query_as};
use uuid::Uuid;

use crate::AuthedToken;

#[async_trait::async_trait]
pub trait AuthedTokenRepository: Send + Sync {
    async fn get_authed_token(&self, id: Uuid) -> anyhow::Result<AuthedToken>;
    async fn delete_authed_token(&self, id: Uuid) -> anyhow::Result<()>;
    async fn delete_authed_token_by_origin_ip(&self, id: Uuid) -> anyhow::Result<()>;
}

#[derive(Clone)]
pub struct AuthedTokenRepositoryImpl(pub sqlx::MySqlPool);

impl AuthedTokenRepositoryImpl {
    pub fn new(pool: sqlx::MySqlPool) -> Self {
        Self(pool)
    }
}

#[async_trait::async_trait]
impl AuthedTokenRepository for AuthedTokenRepositoryImpl {
    async fn get_authed_token(&self, id: Uuid) -> anyhow::Result<AuthedToken> {
        let query = query_as!(
            AuthedToken,
            r#"
            SELECT
                id AS "id!: Uuid",
                token,
                origin_ip,
                reduced_origin_ip,
                writing_ua,
                authed_ua,
                created_at,
                authed_at,
                validity AS "validity!: bool",
                last_wrote_at
            FROM
                authed_tokens
            WHERE
                id = ?
            "#,
            id.as_bytes().to_vec(),
        );

        let authed_token = query.fetch_one(&self.0).await?;

        Ok(authed_token)
    }

    async fn delete_authed_token(&self, id: Uuid) -> anyhow::Result<()> {
        let query = query!(
            r#"
            UPDATE
                authed_tokens
            SET
                validity = 0
            WHERE
                id = ?
        "#,
            id.as_bytes().to_vec(),
        );

        query.execute(&self.0).await?;

        Ok(())
    }

    async fn delete_authed_token_by_origin_ip(&self, id: Uuid) -> anyhow::Result<()> {
        let query = query!(
            r#"
            UPDATE
                authed_tokens
            SET
                validity = 0
            WHERE
                id IN (
                    SELECT id FROM (
                        SELECT
                            id
                        FROM
                            authed_tokens
                        WHERE
                            origin_ip = ?
                    ) tmp      
                )
        "#,
            id.as_bytes().to_vec(),
        );

        query.execute(&self.0).await?;

        Ok(())
    }
}
