use anyhow::Context as _;
use eddist_entity::{authed_token, idp, user, user_authed_token, user_idp_binding};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction, EntityTrait,
    FromQueryResult, IsolationLevel, JoinType, Order, QueryFilter, QueryOrder, QuerySelect,
    RelationTrait, TransactionTrait,
    sea_query::{Expr, InsertStatement, NullOrdering, Query},
};
use uuid::Uuid;

use crate::domain::user::{User, UserIdp};

#[async_trait::async_trait]
pub trait UserRepository: Send + Sync + 'static {
    async fn get_user_by_id(&self, id: Uuid) -> anyhow::Result<Option<User>>;
    /// `idp_sub` must already be hashed (see `hash_idp_sub`).
    async fn get_user_by_idp_sub(
        &self,
        idp_name: &str,
        idp_sub: &str,
    ) -> anyhow::Result<Option<User>>;
    async fn get_all_authed_tokens_by_user_id(&self, user_id: Uuid) -> anyhow::Result<Vec<Uuid>>;
    async fn get_valid_authed_token_by_user_id(
        &self,
        user_id: Uuid,
    ) -> anyhow::Result<Option<String>>;
    async fn is_user_binded_authed_token(&self, authed_token_id: Uuid) -> anyhow::Result<bool>;
    /// Create the user, IdP binding and token binding in one transaction.
    async fn register_user_and_bind_token(
        &self,
        user: CreatingUser,
        token_id: Uuid,
    ) -> anyhow::Result<()>;
    /// Lock the user, bind the token and inherit the canonical seed atomically.
    async fn bind_user_authed_token(
        &self,
        user_id: Uuid,
        authed_token_id: Uuid,
    ) -> anyhow::Result<()>;
}

#[derive(Debug, Clone)]
pub struct UserRepositoryImpl {
    db: DatabaseConnection,
}

impl UserRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

fn user_idp_select() -> sea_orm::Select<user::Entity> {
    user::Entity::find()
        .select_only()
        .column_as(user::Column::Id, "user_id")
        .column_as(user::Column::UserName, "user_name")
        .column_as(user::Column::Enabled, "user_enabled")
        .column_as(user::Column::CreatedAt, "user_created_at")
        .column_as(user::Column::UpdatedAt, "user_updated_at")
        .column_as(idp::Column::Id, "idp_id")
        .column_as(idp::Column::IdpName, "idp_name")
        .column_as(idp::Column::IdpDisplayName, "idp_display_name")
        .column_as(user_idp_binding::Column::IdpSub, "idp_sub")
        .column_as(user_idp_binding::Column::CreatedAt, "idp_bind_created_at")
        .column_as(user_idp_binding::Column::UpdatedAt, "idp_bind_updated_at")
        .join(JoinType::InnerJoin, user::Relation::UserIdpBinding.def())
        .join(JoinType::InnerJoin, user_idp_binding::Relation::Idp.def())
}

fn locked_user(user_id: Uuid) -> sea_orm::Select<user::Entity> {
    user::Entity::find()
        .select_only()
        .column(user::Column::Id)
        .filter(user::Column::Id.eq(user_id))
        .lock_exclusive()
}

// Keep NOW()'s database clock and default second precision. ActiveModel values
// would require an application timestamp for these required columns.
fn user_insert(input: &CreatingUser) -> InsertStatement {
    Query::insert()
        .into_table(user::Entity)
        .columns([
            user::Column::Id,
            user::Column::UserName,
            user::Column::CreatedAt,
            user::Column::UpdatedAt,
        ])
        .values_panic([
            Expr::value(input.user_id),
            Expr::value(input.user_name.clone()),
            Expr::current_timestamp(),
            Expr::current_timestamp(),
        ])
        .to_owned()
}

fn idp_binding_insert(input: &CreatingUser) -> InsertStatement {
    use user_idp_binding::Column;
    Query::insert()
        .into_table(user_idp_binding::Entity)
        .columns([
            Column::Id,
            Column::UserId,
            Column::IdpId,
            Column::IdpSub,
            Column::CreatedAt,
            Column::UpdatedAt,
        ])
        .values_panic([
            Expr::value(Uuid::now_v7()),
            Expr::value(input.user_id),
            Expr::value(input.idp_id),
            Expr::value(input.idp_sub.clone()),
            Expr::current_timestamp(),
            Expr::current_timestamp(),
        ])
        .to_owned()
}

fn token_binding_insert(user_id: Uuid, token_id: Uuid) -> InsertStatement {
    use user_authed_token::Column;
    Query::insert()
        .into_table(user_authed_token::Entity)
        .columns([
            Column::Id,
            Column::UserId,
            Column::AuthedTokenId,
            Column::CreatedAt,
            Column::UpdatedAt,
        ])
        .values_panic([
            Expr::value(Uuid::now_v7()),
            Expr::value(user_id),
            Expr::value(token_id),
            Expr::current_timestamp(),
            Expr::current_timestamp(),
        ])
        .to_owned()
}

async fn bind_in_transaction(
    tx: &DatabaseTransaction,
    user_id: Uuid,
    token_id: Uuid,
) -> anyhow::Result<()> {
    log::info!(
        "insert: bind_user_authed_token: user_id: {}, authed_token_id: {}",
        user_id,
        token_id
    );
    // Lock order is user -> binding/token. Keep this before the binding INSERT.
    if locked_user(user_id)
        .into_tuple::<Uuid>()
        .one(tx)
        .await?
        .is_none()
    {
        anyhow::bail!("user {user_id} not found");
    }
    tx.execute(&token_binding_insert(user_id, token_id)).await?;
    log::info!(
        "update: bind_user_authed_token: user_id: {}, authed_token_id: {}",
        user_id,
        token_id
    );
    // Preserve the existing selection order, including its unspecified tie order.
    let seed = authed_token::Entity::find()
        .select_only()
        .column(authed_token::Column::AuthorIdSeed)
        .join(
            JoinType::InnerJoin,
            authed_token::Relation::UserAuthedToken.def(),
        )
        .filter(user_authed_token::Column::UserId.eq(user_id))
        .order_by_asc(user_authed_token::Column::CreatedAt)
        .into_tuple::<Vec<u8>>()
        .one(tx)
        .await?;
    let mut update = authed_token::Entity::update_many()
        .col_expr(authed_token::Column::RegisteredUserId, Expr::value(user_id))
        .filter(authed_token::Column::Id.eq(token_id));
    if let Some(seed) = seed {
        update = update.col_expr(authed_token::Column::AuthorIdSeed, Expr::value(seed));
    }
    update.exec(tx).await?;
    Ok(())
}

#[async_trait::async_trait]
impl UserRepository for UserRepositoryImpl {
    async fn get_user_by_id(&self, id: Uuid) -> anyhow::Result<Option<User>> {
        let rows = user_idp_select()
            .filter(user::Column::Id.eq(id))
            .into_model::<UserIdpSelection>()
            .all(&self.db)
            .await?;
        Ok(assemble_user_from_user_idp_rows(rows))
    }

    async fn get_user_by_idp_sub(
        &self,
        idp_name: &str,
        idp_sub: &str,
    ) -> anyhow::Result<Option<User>> {
        let rows = user_idp_select()
            .filter(idp::Column::IdpName.eq(idp_name))
            .filter(user_idp_binding::Column::IdpSub.eq(idp_sub))
            .into_model::<UserIdpSelection>()
            .all(&self.db)
            .await?;
        Ok(assemble_user_from_user_idp_rows(rows))
    }

    async fn get_all_authed_tokens_by_user_id(&self, user_id: Uuid) -> anyhow::Result<Vec<Uuid>> {
        Ok(user_authed_token::Entity::find()
            .select_only()
            .column(user_authed_token::Column::AuthedTokenId)
            .filter(user_authed_token::Column::UserId.eq(user_id))
            .order_by_asc(user_authed_token::Column::CreatedAt)
            .into_tuple::<Uuid>()
            .all(&self.db)
            .await?)
    }

    async fn get_valid_authed_token_by_user_id(
        &self,
        user_id: Uuid,
    ) -> anyhow::Result<Option<String>> {
        Ok(authed_token::Entity::find()
            .select_only()
            .column(authed_token::Column::Token)
            .join(
                JoinType::InnerJoin,
                authed_token::Relation::UserAuthedToken.def(),
            )
            .filter(user_authed_token::Column::UserId.eq(user_id))
            .filter(authed_token::Column::Validity.eq(true))
            // MySQL sorts NULL activation times last for DESC; spell it out for other backends.
            .order_by_with_nulls(
                authed_token::Column::AuthedAt,
                Order::Desc,
                NullOrdering::Last,
            )
            .into_tuple::<String>()
            .one(&self.db)
            .await?)
    }

    async fn is_user_binded_authed_token(&self, authed_token_id: Uuid) -> anyhow::Result<bool> {
        authed_token::Entity::find()
            .select_only()
            .expr_as(
                authed_token::Column::RegisteredUserId.is_not_null(),
                "is_binded",
            )
            .filter(authed_token::Column::Id.eq(authed_token_id))
            .into_tuple::<bool>()
            .one(&self.db)
            .await?
            .with_context(|| format!("authed token {authed_token_id} not found"))
    }

    async fn register_user_and_bind_token(
        &self,
        user: CreatingUser,
        token_id: Uuid,
    ) -> anyhow::Result<()> {
        let tx = self
            .db
            .begin_with_config(Some(IsolationLevel::ReadCommitted), None)
            .await?;
        tx.execute(&user_insert(&user)).await?;
        tx.execute(&idp_binding_insert(&user)).await?;
        bind_in_transaction(&tx, user.user_id, token_id).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn bind_user_authed_token(
        &self,
        user_id: Uuid,
        authed_token_id: Uuid,
    ) -> anyhow::Result<()> {
        let tx = self
            .db
            .begin_with_config(Some(IsolationLevel::ReadCommitted), None)
            .await?;
        bind_in_transaction(&tx, user_id, authed_token_id).await?;
        tx.commit().await?;
        Ok(())
    }
}

fn assemble_user_from_user_idp_rows(user_idp_rows: Vec<UserIdpSelection>) -> Option<User> {
    user_idp_rows
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
                    enabled: row.user_enabled,
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
        })
}

#[derive(FromQueryResult)]
struct UserIdpSelection {
    pub user_id: Uuid,
    pub user_name: String,
    pub user_enabled: bool,
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
    /// Must already be hashed (see `hash_idp_sub`), not the raw IdP `sub` claim.
    pub idp_sub: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{DbBackend, QueryTrait};

    #[test]
    fn user_lock_is_exclusive_in_both_dialects() {
        let query = locked_user(Uuid::nil());
        assert_eq!(
            query.build(DbBackend::MySql).sql,
            "SELECT `users`.`id` FROM `users` WHERE `users`.`id` = ? FOR UPDATE"
        );
        assert_eq!(
            query.build(DbBackend::Postgres).sql,
            "SELECT \"users\".\"id\" FROM \"users\" WHERE \"users\".\"id\" = $1 FOR UPDATE"
        );
    }

    #[test]
    fn registration_timestamps_remain_database_expressions() {
        let input = CreatingUser {
            user_id: Uuid::nil(),
            user_name: "user".into(),
            idp_id: Uuid::nil(),
            idp_sub: "hashed".into(),
        };
        for (query, expected_parameters) in [
            (user_insert(&input), 2),
            (idp_binding_insert(&input), 4),
            (token_binding_insert(input.user_id, Uuid::nil()), 3),
        ] {
            for backend in [DbBackend::MySql, DbBackend::Postgres] {
                let statement = backend.build(&query);
                assert_eq!(statement.sql.matches("CURRENT_TIMESTAMP").count(), 2);
                assert_eq!(statement.values.unwrap().0.len(), expected_parameters);
            }
        }
    }
}
