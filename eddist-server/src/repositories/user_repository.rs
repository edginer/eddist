use sqlx::{MySql, MySqlPool, Transaction};
use uuid::Uuid;

use crate::{
    domain::user::user::{User, UserIdp},
    transaction_repository,
};

#[async_trait::async_trait]
pub trait UserRepository: Send + Sync + 'static {
    async fn get_user_by_id(&self, id: Uuid) -> anyhow::Result<Option<User>>;
    async fn get_user_by_idp_sub(
        &self,
        idp_name: &str,
        idp_sub: &str,
    ) -> anyhow::Result<Option<User>>;
    async fn is_user_binded_authed_token(&self, authed_token_id: Uuid) -> anyhow::Result<bool>;
    async fn create_user_with_idp<'a>(
        &'a self,
        user: CreatingUser,
        tx: Transaction<'a, MySql>,
    ) -> anyhow::Result<Transaction<'a, MySql>>;
    async fn bind_user_authed_token<'a>(
        &'a self,
        user_id: Uuid,
        authed_token_id: Uuid,
        tx: Transaction<'a, MySql>,
    ) -> anyhow::Result<Transaction<'a, MySql>>;
}

#[derive(Debug, Clone)]
pub struct UserRepositoryImpl {
    pool: MySqlPool,
}

impl UserRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

transaction_repository!(UserRepositoryImpl, pool, MySql);

#[async_trait::async_trait]
impl UserRepository for UserRepositoryImpl {
    async fn get_user_by_id(&self, id: Uuid) -> anyhow::Result<Option<User>> {
        let user_idps = sqlx::query_as!(
            UserIdpSelection,
            r#"
            SELECT
                us.id AS "user_id: Uuid",
                us.user_name AS "user_name: String",
                us.created_at AS "user_created_at: chrono::NaiveDateTime",
                us.updated_at AS "user_updated_at: chrono::NaiveDateTime",
                idps.id AS "idp_id: Uuid",
                idps.idp_name AS "idp_name: String",
                idps.idp_display_name AS "idp_display_name: String",
                uib.idp_sub AS "idp_sub: String",
                uib.created_at AS "idp_bind_created_at: chrono::NaiveDateTime",
                uib.updated_at AS "idp_bind_updated_at: chrono::NaiveDateTime"
            FROM users AS us
            JOIN user_idp_bindings AS uib ON us.id = uib.user_id
            JOIN idps AS idps ON uib.idp_id = idps.id
            WHERE us.id = ?
            "#,
            id
        )
        .fetch_all(&self.pool)
        .await?;

        let user = user_idps
            .into_iter()
            .fold(None, |mut acc: Option<User>, row| {
                if let Some(user) = acc.as_mut() {
                    user.idps.push(UserIdp {
                        idp_id: row.idp_id,
                        idp_name: row.idp_name,
                        idp_display_name: row.idp_display_name,
                        idp_sub: row.idp_sub,
                        created_at: row.idp_bind_created_at,
                        updated_at: row.idp_bind_updated_at,
                    });
                } else {
                    acc = Some(User {
                        id: row.user_id,
                        user_name: row.user_name,
                        idps: vec![UserIdp {
                            idp_id: row.idp_id,
                            idp_name: row.idp_name,
                            idp_display_name: row.idp_display_name,
                            idp_sub: row.idp_sub,
                            created_at: row.idp_bind_created_at,
                            updated_at: row.idp_bind_updated_at,
                        }],
                        created_at: row.user_created_at,
                        updated_at: row.user_updated_at,
                    });
                }

                acc
            });

        Ok(user)
    }

    async fn get_user_by_idp_sub(
        &self,
        idp_name: &str,
        idp_sub: &str,
    ) -> anyhow::Result<Option<User>> {
        let user_idps = sqlx::query_as!(
            UserIdpSelection,
            r#"
            SELECT
                us.id AS "user_id: Uuid",
                us.user_name AS "user_name: String",
                us.created_at AS "user_created_at: chrono::NaiveDateTime",
                us.updated_at AS "user_updated_at: chrono::NaiveDateTime",
                idps.id AS "idp_id: Uuid",
                idps.idp_name AS "idp_name: String",
                idps.idp_display_name AS "idp_display_name: String",
                uib.idp_sub AS "idp_sub: String",
                uib.created_at AS "idp_bind_created_at: chrono::NaiveDateTime",
                uib.updated_at AS "idp_bind_updated_at: chrono::NaiveDateTime"
            FROM users AS us
            JOIN user_idp_bindings AS uib ON us.id = uib.user_id
            JOIN idps AS idps ON uib.idp_id = idps.id
            WHERE idps.idp_name = ? AND uib.idp_sub = ?
            "#,
            idp_name,
            idp_sub
        )
        .fetch_all(&self.pool)
        .await?;

        let user = user_idps
            .into_iter()
            .fold(None, |mut acc: Option<User>, row| {
                if let Some(user) = acc.as_mut() {
                    user.idps.push(UserIdp {
                        idp_id: row.idp_id,
                        idp_name: row.idp_name,
                        idp_display_name: row.idp_display_name,
                        idp_sub: row.idp_sub,
                        created_at: row.idp_bind_created_at,
                        updated_at: row.idp_bind_updated_at,
                    });
                } else {
                    acc = Some(User {
                        id: row.user_id,
                        user_name: row.user_name,
                        idps: vec![UserIdp {
                            idp_id: row.idp_id,
                            idp_name: row.idp_name,
                            idp_display_name: row.idp_display_name,
                            idp_sub: row.idp_sub,
                            created_at: row.idp_bind_created_at,
                            updated_at: row.idp_bind_updated_at,
                        }],
                        created_at: row.user_created_at,
                        updated_at: row.user_updated_at,
                    });
                }

                acc
            });

        Ok(user)
    }

    /// !!! You need to call begin / commit / rollback outside of this function !!!
    async fn create_user_with_idp<'a>(
        &'a self,
        user: CreatingUser,
        mut tx: Transaction<'a, MySql>,
    ) -> anyhow::Result<Transaction<'a, MySql>> {
        sqlx::query!(
            r#"
            INSERT INTO users (id, user_name, created_at, updated_at)
            VALUES (?, ?, NOW(), NOW())
            "#,
            user.user_id,
            user.user_name
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
            INSERT INTO user_idp_bindings (id, user_id, idp_id, idp_sub, created_at, updated_at)
            VALUES (?, ?, ?, ?, NOW(), NOW())
            "#,
            Uuid::now_v7(),
            user.user_id,
            user.idp_id,
            user.idp_sub
        )
        .execute(&mut *tx)
        .await?;

        Ok(tx)
    }

    async fn is_user_binded_authed_token(&self, authed_token_id: Uuid) -> anyhow::Result<bool> {
        let is_binded = sqlx::query!(
            r#"
            SELECT registered_user_id IS NOT NULL AS is_binded
            FROM authed_tokens
            WHERE id = ?
            "#,
            authed_token_id
        )
        .fetch_one(&self.pool)
        .await?
        .is_binded
            > 0;

        Ok(is_binded)
    }

    /// !!! You need to call begin / commit / rollback outside of this function !!!
    async fn bind_user_authed_token<'a>(
        &'a self,
        user_id: Uuid,
        authed_token_id: Uuid,
        mut tx: Transaction<'a, MySql>,
    ) -> anyhow::Result<Transaction<'a, MySql>> {
        log::info!(
            "insert: bind_user_authed_token: user_id: {}, authed_token_id: {}",
            user_id,
            authed_token_id
        );

        sqlx::query!(
            r#"
            INSERT INTO user_authed_tokens (id, user_id, authed_token_id, created_at, updated_at)
            VALUES (?, ?, ?, NOW(), NOW())
            "#,
            Uuid::now_v7(),
            user_id,
            authed_token_id
        )
        .execute(&mut *tx)
        .await?;

        log::info!(
            "update: bind_user_authed_token: user_id: {}, authed_token_id: {}",
            user_id,
            authed_token_id
        );

        // TODO: It is not good idea that update authed_tokens outside of BbsRepository
        sqlx::query!(
            r#"
            UPDATE authed_tokens
            SET registered_user_id = ?
            WHERE id = ?
            "#,
            user_id,
            authed_token_id
        )
        .execute(&mut *tx)
        .await?;

        Ok(tx)
    }
}

struct UserIdpSelection {
    pub user_id: Uuid,
    pub user_name: String,
    pub user_created_at: chrono::NaiveDateTime,
    pub user_updated_at: chrono::NaiveDateTime,
    pub idp_id: Uuid,
    pub idp_name: String,
    pub idp_display_name: String,
    pub idp_sub: String,
    pub idp_bind_created_at: chrono::NaiveDateTime,
    pub idp_bind_updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct CreatingUser {
    pub user_id: Uuid,
    pub user_name: String,
    pub idp_id: Uuid,
    pub idp_sub: String,
}
