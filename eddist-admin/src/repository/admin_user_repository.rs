use crate::entity::{authed_token, idp, user, user_authed_token, user_idp_binding};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    TransactionTrait,
};
use std::collections::HashMap;
use uuid::Uuid;

use crate::models::{User, UserIdpBinding};

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
pub struct AdminUserRepositoryImpl(DatabaseConnection);

impl AdminUserRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self(db)
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

        // Resolve the token filter first so the final user query can remain a typed Entity query.
        let token_user_ids = if let Some(authed_token_id) = authed_token_id {
            user_authed_token::Entity::find()
                .filter(user_authed_token::Column::AuthedTokenId.eq(authed_token_id))
                .all(&self.0)
                .await?
                .into_iter()
                .map(|relation| relation.user_id)
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        let mut condition = Condition::any();
        if let Some(user_id) = user_id {
            condition = condition.add(user::Column::Id.eq(user_id));
        }
        if let Some(user_name) = user_name {
            condition = condition.add(user::Column::UserName.eq(user_name));
        }
        if authed_token_id.is_some() {
            condition = condition.add(user::Column::Id.is_in(token_user_ids));
        }

        let users = user::Entity::find()
            .filter(condition)
            .order_by_asc(user::Column::UserName)
            .all(&self.0)
            .await?;
        if users.is_empty() {
            return Ok(vec![]);
        }

        let user_ids = users.iter().map(|model| model.id).collect::<Vec<_>>();
        let bindings = user_idp_binding::Entity::find()
            .filter(user_idp_binding::Column::UserId.is_in(user_ids.clone()))
            .find_also_related(idp::Entity)
            .all(&self.0)
            .await?;

        let mut idp_bindings_by_user = HashMap::<Uuid, Vec<UserIdpBinding>>::new();
        for (binding, idp) in bindings {
            // A binding whose IdP row is gone is skipped, matching the pre-SeaORM
            // LEFT JOIN, which dropped rows with a NULL idp_name.
            if let Some(idp) = idp {
                idp_bindings_by_user
                    .entry(binding.user_id)
                    .or_default()
                    .push(UserIdpBinding {
                        id: binding.id,
                        user_id: binding.user_id,
                        idp_name: idp.idp_name,
                        idp_sub: binding.idp_sub,
                    });
            }
        }

        let token_relations = user_authed_token::Entity::find()
            .filter(user_authed_token::Column::UserId.is_in(user_ids))
            .all(&self.0)
            .await?;
        let mut token_ids_by_user = HashMap::<Uuid, Vec<Uuid>>::new();
        for relation in token_relations {
            token_ids_by_user
                .entry(relation.user_id)
                .or_default()
                .push(relation.authed_token_id);
        }

        Ok(users
            .into_iter()
            .map(|model| User {
                id: model.id,
                user_name: model.user_name,
                enabled: model.enabled,
                idp_bindings: idp_bindings_by_user.remove(&model.id).unwrap_or_default(),
                authed_token_ids: token_ids_by_user.remove(&model.id).unwrap_or_default(),
            })
            .collect())
    }

    async fn update_user_status(&self, user_id: Uuid, enabled: bool) -> anyhow::Result<()> {
        let tx = self.0.begin().await?;

        user::Entity::update_many()
            .col_expr(user::Column::Enabled, Expr::value(enabled))
            .filter(user::Column::Id.eq(user_id))
            .exec(&tx)
            .await?;

        let token_ids = user_authed_token::Entity::find()
            .filter(user_authed_token::Column::UserId.eq(user_id))
            .all(&tx)
            .await?
            .into_iter()
            .map(|relation| relation.authed_token_id)
            .collect::<Vec<_>>();
        if !token_ids.is_empty() {
            authed_token::Entity::update_many()
                .col_expr(authed_token::Column::Validity, Expr::value(enabled))
                .filter(authed_token::Column::Id.is_in(token_ids))
                .exec(&tx)
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }
}
