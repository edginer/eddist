use chrono::{DateTime, Utc};
use eddist_core::domain::{
    authed_token_backup::APP_INTERNAL_TOKEN_PREFIX,
    ip_addr::{IpAddr, ReducedIpAddr},
};
use eddist_entity::{app_attest_key, app_credential, authed_token};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    QueryFilter, QuerySelect, SqlErr, TransactionTrait, sea_query::Expr,
};
use sha2::{Digest, Sha512};
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct AppCredentialsRepository(DatabaseConnection);

pub(crate) struct RegistrationClient<'a> {
    pub ip: &'a str,
    pub user_agent: &'a str,
    pub asn: i32,
    pub require_user_registration: bool,
}

pub(crate) enum RefreshResult {
    Updated(DateTime<Utc>),
    Revoked,
    Rejected,
}

impl AppCredentialsRepository {
    pub(crate) fn new(db: DatabaseConnection) -> Self {
        Self(db)
    }

    pub(crate) async fn find_by_attempt(
        &self,
        platform: &str,
        scope_hash: &[u8; 32],
        key_hash: &[u8; 32],
    ) -> Result<Option<app_credential::Model>, DbErr> {
        app_credential::Entity::find()
            .filter(app_credential::Column::Platform.eq(platform))
            .filter(app_credential::Column::ScopeHash.eq(scope_hash.to_vec()))
            .filter(app_credential::Column::RegistrationKeyHash.eq(key_hash.to_vec()))
            .one(&self.0)
            .await
    }

    pub(crate) async fn find_by_bearer(
        &self,
        platform: &str,
        scope_hash: &[u8; 32],
        bearer_hash: &[u8; 32],
    ) -> Result<Option<app_credential::Model>, DbErr> {
        app_credential::Entity::find()
            .filter(app_credential::Column::Platform.eq(platform))
            .filter(app_credential::Column::ScopeHash.eq(scope_hash.to_vec()))
            .filter(app_credential::Column::BearerHash.eq(bearer_hash.to_vec()))
            .one(&self.0)
            .await
    }

    pub(crate) async fn find_app_attest_key(
        &self,
        id: Uuid,
    ) -> Result<Option<app_attest_key::Model>, DbErr> {
        app_attest_key::Entity::find_by_id(id).one(&self.0).await
    }

    pub(crate) async fn is_valid(&self, id: Uuid) -> Result<bool, DbErr> {
        Ok(authed_token::Entity::find_by_id(id)
            .filter(authed_token::Column::Validity.eq(true))
            .one(&self.0)
            .await?
            .is_some())
    }

    pub(crate) async fn issue(
        &self,
        credential: app_credential::Model,
        app_attest_key: Option<app_attest_key::Model>,
        client: RegistrationClient<'_>,
    ) -> Result<bool, DbErr> {
        let id = credential.authed_token_id;
        let registered_at = credential.registered_at.naive_utc();
        let reduced_ip = ReducedIpAddr::from(IpAddr::new(client.ip.to_owned())).to_string();
        let txn = self.0.begin().await?;
        authed_token::Entity::insert(authed_token::ActiveModel {
            id: Set(id),
            // Existing moderation and user linkage use the authed_tokens row.
            // This internal value is never presented as the app's bearer token.
            token: Set(format!("{APP_INTERNAL_TOKEN_PREFIX}{}", Uuid::new_v4())),
            origin_ip: Set(client.ip.to_owned()),
            reduced_origin_ip: Set(reduced_ip.clone()),
            writing_ua: Set(client.user_agent.to_owned()),
            authed_ua: Set(Some(client.user_agent.to_owned())),
            asn_num: Set(client.asn),
            auth_code: Set(String::new()),
            created_at: Set(registered_at),
            authed_at: Set(Some(registered_at)),
            validity: Set(true),
            require_user_registration: Set(client.require_user_registration),
            author_id_seed: Set(Sha512::digest(&reduced_ip).to_vec()),
            is_app: Set(true),
            ..Default::default()
        })
        .exec_without_returning(&txn)
        .await?;

        let result: Result<(), DbErr> = async {
            app_credential::Entity::insert(app_credential::ActiveModel::from(credential))
                .exec_without_returning(&txn)
                .await?;
            if let Some(key) = app_attest_key {
                app_attest_key::Entity::insert(app_attest_key::ActiveModel::from(key))
                    .exec_without_returning(&txn)
                    .await?;
            }
            Ok(())
        }
        .await;
        match result {
            Ok(()) => {
                txn.commit().await?;
                Ok(true)
            }
            Err(err) if matches!(err.sql_err(), Some(SqlErr::UniqueConstraintViolation(_))) => {
                txn.rollback().await?;
                Ok(false)
            }
            Err(err) => Err(err),
        }
    }

    pub(crate) async fn refresh(
        &self,
        id: Uuid,
        counter: Option<u32>,
        verified_at: DateTime<Utc>,
        reattest_after: DateTime<Utc>,
    ) -> Result<RefreshResult, DbErr> {
        let txn = self.0.begin().await?;
        // Serialize the refresh with moderator updates before locking child rows.
        let token = authed_token::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?;
        if !token.is_some_and(|token| token.validity) {
            txn.rollback().await?;
            return Ok(RefreshResult::Revoked);
        }
        let Some(credential) = app_credential::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
        else {
            txn.rollback().await?;
            return Ok(RefreshResult::Rejected);
        };
        if let Some(counter) = counter {
            let update = app_attest_key::Entity::update_many()
                .filter(app_attest_key::Column::AuthedTokenId.eq(id))
                .filter(app_attest_key::Column::Counter.lt(counter))
                .col_expr(app_attest_key::Column::Counter, Expr::value(counter))
                .exec(&txn)
                .await?;
            if update.rows_affected == 0 {
                txn.rollback().await?;
                return Ok(RefreshResult::Rejected);
            }
        }
        let deadline = reattest_after.max(credential.reattest_after);
        let verified_at = verified_at.max(credential.verified_at);
        if verified_at > credential.verified_at || deadline > credential.reattest_after {
            let mut active = app_credential::ActiveModel::from(credential);
            active.verified_at = Set(verified_at);
            active.reattest_after = Set(deadline);
            active.update(&txn).await?;
        }
        txn.commit().await?;
        Ok(RefreshResult::Updated(deadline))
    }
}
