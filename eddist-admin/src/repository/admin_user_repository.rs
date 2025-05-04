use std::collections::HashMap;

use uuid::Uuid;

use crate::{User, UserIdpBinding};

#[async_trait::async_trait]
pub trait AdminUserRepository: Send + Sync {
    async fn search_users(
        &self,
        user_id: Option<Uuid>,
        user_name: Option<String>,
        authed_token_id: Option<Uuid>,
    ) -> anyhow::Result<Vec<User>>;
    async fn update_user_status(&self, user_id: Uuid, enabled: bool) -> anyhow::Result<()>;
}

#[derive(Clone)]
pub struct AdminUserRepositoryImpl(pub sqlx::MySqlPool);

impl AdminUserRepositoryImpl {
    pub fn new(pool: sqlx::MySqlPool) -> Self {
        Self(pool)
    }
}

#[async_trait::async_trait]
impl AdminUserRepository for AdminUserRepositoryImpl {
    async fn search_users(
        &self,
        user_id: Option<Uuid>,
        user_name: Option<String>,
        authed_token_id: Option<Uuid>,
    ) -> anyhow::Result<Vec<User>> {
        if user_id.is_none() && user_name.is_none() && authed_token_id.is_none() {
            return Ok(vec![]);
        }

        let mut sets = Vec::new();
        let mut values = Vec::new();

        if let Some(user_id) = user_id {
            sets.push("u.id = UUID_TO_BIN(?)");
            values.push(user_id.to_string());
        }
        if let Some(user_name) = user_name {
            sets.push("u.user_name = ?");
            values.push(user_name);
        }
        if authed_token_id.is_some() {
            unimplemented!("search_users by authed_token_id");
        }
        let query = format!(
            r#"
        SELECT
            u.id AS user_id,
            u.user_name AS user_name,
            u.enabled AS enabled,
            ub.id AS idp_binding_id,
            ub.idp_sub AS idp_sub,
            i.idp_name AS idp_name
        FROM
            users AS u
        JOIN
            user_idp_bindings AS ub
        ON
            u.id = ub.user_id
        JOIN
            idps AS i
        ON
            ub.idp_id = i.id
        WHERE {}
        "#,
            sets.join(" OR ")
        );

        let mut query = sqlx::query_as::<_, UserIdpsSelection>(&query);
        for value in values {
            query = query.bind(value);
        }
        let users = query.fetch_all(&self.0).await?;

        let user_id = users
            .iter()
            .map(|x| x.user_id)
            .next()
            .ok_or_else(|| anyhow::anyhow!("user not found"))?;

        // Get authed tokens associated with the user using `user_authed_tokens` table
        let authed_token_ids = sqlx::query!(
            r#"
            SELECT
                authed_token_id AS "token_id!: Uuid"
            FROM
                user_authed_tokens
            WHERE
                user_id = ?
        "#,
            user_id
        )
        .fetch_all(&self.0)
        .await?
        .into_iter()
        .map(|x| x.token_id)
        .collect::<Vec<_>>();

        Ok(users
            .into_iter()
            .fold(HashMap::new(), |mut acc: HashMap<Uuid, User>, user| {
                let user_id = user.user_id;
                if let Some(existing_user) = acc.get_mut(&user_id) {
                    existing_user.idp_bindings.push(UserIdpBinding {
                        id: user.idp_binding_id,
                        user_id,
                        idp_name: user.idp_name,
                        idp_sub: user.idp_sub,
                    });
                } else {
                    acc.insert(
                        user_id,
                        User {
                            id: user_id,
                            user_name: user.user_name,
                            enabled: user.enabled,
                            idp_bindings: vec![UserIdpBinding {
                                id: user.idp_binding_id,
                                user_id,
                                idp_name: user.idp_name,
                                idp_sub: user.idp_sub,
                            }],
                            authed_token_ids: authed_token_ids.clone(),
                        },
                    );
                }
                acc
            })
            .into_values()
            .collect())
    }

    async fn update_user_status(&self, user_id: Uuid, enabled: bool) -> anyhow::Result<()> {
        let mut tx = self.0.begin().await?;

        let query = sqlx::query!(
            r#"
            UPDATE
                users
            SET
                enabled = ?
            WHERE
                id = ?
        "#,
            enabled,
            user_id
        );

        query.execute(&mut *tx).await?;

        // Enable/Disable all authed tokens associated with the user
        let query = sqlx::query!(
            r#"
            UPDATE
                authed_tokens
            SET
                validity = ?
            WHERE
                id IN (
                    SELECT
                        authed_token_id
                    FROM
                        user_authed_tokens
                    WHERE
                        user_id = ?
                )
        "#,
            enabled,
            user_id
        );

        query.execute(&mut *tx).await?;

        tx.commit().await?;

        Ok(())
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserIdpsSelection {
    pub user_id: Uuid,
    pub user_name: String,
    pub enabled: bool,
    pub idp_name: String,
    pub idp_sub: String,
    pub idp_binding_id: Uuid,
}
