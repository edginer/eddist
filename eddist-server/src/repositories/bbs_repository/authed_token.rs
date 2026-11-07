use chrono::{DateTime, NaiveDateTime, Utc};
use eddist_core::domain::ip_addr::{IpAddr, ReducedIpAddr};
use eddist_entity::{authed_token, db_time::truncate_to_millis};
use sea_orm::{
    ActiveValue::Set, ColumnTrait, EntityTrait, FromQueryResult, QueryFilter, QuerySelect,
    sea_query::Expr,
};
use uuid::Uuid;

use crate::domain::authed_token::AuthedToken;

use super::BbsRepositoryImpl;

#[async_trait::async_trait]
pub trait AuthedTokenRepository: Send + Sync + 'static {
    async fn get_authed_token(&self, token: &str) -> anyhow::Result<Option<AuthedToken>>;
    async fn get_authed_token_by_id(&self, id: Uuid) -> anyhow::Result<Option<AuthedToken>>;
    async fn get_authed_token_by_origin_ip_and_auth_code(
        &self,
        ip: &str,
        auth_code: &str,
    ) -> anyhow::Result<Option<AuthedToken>>;
    async fn get_unauthed_authed_token_by_auth_code(
        &self,
        auth_code: &str,
    ) -> anyhow::Result<Vec<AuthedToken>>;
    async fn create_authed_token(&self, authed_token: CreatingAuthedToken) -> anyhow::Result<()>;
    async fn activate_authed_status(
        &self,
        token: &str,
        authed_ua: &str,
        authed_time: DateTime<Utc>,
        additional_info: Option<serde_json::Value>,
    ) -> anyhow::Result<()>;
    async fn update_authed_token_last_wrote(
        &self,
        token_id: Uuid,
        last_wrote: DateTime<Utc>,
    ) -> anyhow::Result<()>;
    async fn revoke_authed_token(&self, token: &str) -> anyhow::Result<()>;
    async fn delete_authed_token(&self, token: &str) -> anyhow::Result<()>;
    async fn clear_require_reauth(&self, id: Uuid) -> anyhow::Result<()>;
}

fn token_select() -> sea_orm::Select<authed_token::Entity> {
    use authed_token::Column;
    authed_token::Entity::find().select_only().columns([
        Column::Id,
        Column::Token,
        Column::OriginIp,
        Column::ReducedOriginIp,
        Column::AsnNum,
        Column::WritingUa,
        Column::AuthedUa,
        Column::AuthCode,
        Column::CreatedAt,
        Column::AuthedAt,
        Column::Validity,
        Column::LastWroteAt,
        Column::AuthorIdSeed,
        Column::RequireUserRegistration,
        Column::RegisteredUserId,
        Column::RequireReauth,
    ])
}

#[async_trait::async_trait]
impl AuthedTokenRepository for BbsRepositoryImpl {
    async fn get_authed_token(&self, token: &str) -> anyhow::Result<Option<AuthedToken>> {
        Ok(token_select()
            .filter(authed_token::Column::Token.eq(token))
            .into_model::<SelectionAuthedToken>()
            .one(&self.db)
            .await?
            .map(SelectionAuthedToken::into_authed_token))
    }

    async fn get_authed_token_by_id(&self, id: Uuid) -> anyhow::Result<Option<AuthedToken>> {
        Ok(token_select()
            .filter(authed_token::Column::Id.eq(id))
            .into_model::<SelectionAuthedToken>()
            .one(&self.db)
            .await?
            .map(SelectionAuthedToken::into_authed_token))
    }

    async fn get_authed_token_by_origin_ip_and_auth_code(
        &self,
        reduced_ip: &str,
        auth_code: &str,
    ) -> anyhow::Result<Option<AuthedToken>> {
        Ok(token_select()
            .filter(authed_token::Column::ReducedOriginIp.eq(reduced_ip))
            .filter(authed_token::Column::AuthCode.eq(auth_code))
            .into_model::<SelectionAuthedToken>()
            .one(&self.db)
            .await?
            .map(SelectionAuthedToken::into_authed_token))
    }

    async fn get_unauthed_authed_token_by_auth_code(
        &self,
        auth_code: &str,
    ) -> anyhow::Result<Vec<AuthedToken>> {
        Ok(token_select()
            .filter(authed_token::Column::AuthCode.eq(auth_code))
            .filter(authed_token::Column::Validity.eq(false))
            .into_model::<SelectionAuthedToken>()
            .all(&self.db)
            .await?
            .into_iter()
            .map(SelectionAuthedToken::into_authed_token)
            .collect())
    }

    async fn create_authed_token(&self, token: CreatingAuthedToken) -> anyhow::Result<()> {
        let ip_addr = token.origin_ip.to_string();
        let reduced_ip = ReducedIpAddr::from(token.origin_ip).to_string();
        authed_token::Entity::insert(authed_token::ActiveModel {
            id: Set(token.id),
            token: Set(token.token),
            origin_ip: Set(ip_addr),
            reduced_origin_ip: Set(reduced_ip),
            asn_num: Set(token.asn_num),
            writing_ua: Set(token.writing_ua),
            auth_code: Set(token.auth_code),
            created_at: Set(truncate_to_millis(token.created_at.naive_utc())),
            validity: Set(false),
            author_id_seed: Set(token.author_id_seed),
            require_user_registration: Set(token.require_user_registration),
            ..Default::default()
        })
        .exec_without_returning(&self.db)
        .await?;
        Ok(())
    }

    async fn activate_authed_status(
        &self,
        token: &str,
        authed_ua: &str,
        authed_time: DateTime<Utc>,
        additional_info: Option<serde_json::Value>,
    ) -> anyhow::Result<()> {
        use authed_token::Column;
        authed_token::Entity::update_many()
            .col_expr(Column::Validity, Expr::value(true))
            .col_expr(Column::AuthedUa, Expr::value(authed_ua))
            .col_expr(
                Column::AuthedAt,
                Expr::value(truncate_to_millis(authed_time.naive_utc())),
            )
            .col_expr(Column::AdditionalInfo, Expr::value(additional_info))
            .filter(Column::Token.eq(token))
            .exec(&self.db)
            .await?;
        Ok(())
    }

    async fn update_authed_token_last_wrote(
        &self,
        token_id: Uuid,
        last_wrote: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        authed_token::Entity::update_many()
            .col_expr(
                authed_token::Column::LastWroteAt,
                Expr::value(truncate_to_millis(last_wrote.naive_utc())),
            )
            .filter(authed_token::Column::Id.eq(token_id))
            .exec(&self.db)
            .await?;
        Ok(())
    }

    async fn revoke_authed_token(&self, token: &str) -> anyhow::Result<()> {
        authed_token::Entity::update_many()
            .col_expr(authed_token::Column::Validity, Expr::value(false))
            .filter(authed_token::Column::Token.eq(token))
            .exec(&self.db)
            .await?;
        Ok(())
    }

    async fn delete_authed_token(&self, token: &str) -> anyhow::Result<()> {
        authed_token::Entity::delete_many()
            .filter(authed_token::Column::Token.eq(token))
            .exec(&self.db)
            .await?;
        Ok(())
    }

    async fn clear_require_reauth(&self, id: Uuid) -> anyhow::Result<()> {
        authed_token::Entity::update_many()
            .col_expr(authed_token::Column::RequireReauth, Expr::value(false))
            .filter(authed_token::Column::Id.eq(id))
            .exec(&self.db)
            .await?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CreatingAuthedToken {
    pub id: Uuid,
    pub token: String,
    pub origin_ip: IpAddr,
    pub asn_num: i32,
    pub writing_ua: String,
    pub auth_code: String,
    pub created_at: DateTime<Utc>,
    pub author_id_seed: Vec<u8>,
    pub require_user_registration: bool,
}

#[derive(Debug, FromQueryResult)]
struct SelectionAuthedToken {
    id: Uuid,
    token: String,
    origin_ip: String,
    reduced_origin_ip: String,
    asn_num: i32,
    writing_ua: String,
    authed_ua: Option<String>,
    auth_code: String,
    created_at: NaiveDateTime,
    authed_at: Option<NaiveDateTime>,
    validity: bool,
    last_wrote_at: Option<NaiveDateTime>,
    author_id_seed: Vec<u8>,
    require_user_registration: bool,
    registered_user_id: Option<Uuid>,
    require_reauth: bool,
}

impl SelectionAuthedToken {
    fn into_authed_token(self) -> AuthedToken {
        AuthedToken {
            id: self.id,
            token: self.token,
            origin_ip: IpAddr::new(self.origin_ip),
            reduced_ip: ReducedIpAddr::from(self.reduced_origin_ip),
            asn_num: self.asn_num,
            writing_ua: self.writing_ua,
            authed_ua: self.authed_ua,
            auth_code: self.auth_code,
            created_at: self.created_at.and_utc(),
            authed_at: self.authed_at.map(|x| x.and_utc()),
            validity: self.validity,
            last_wrote_at: self.last_wrote_at.map(|x| x.and_utc()),
            author_id_seed: self.author_id_seed,
            require_user_registration: self.require_user_registration,
            registered_user_id: self.registered_user_id,
            require_reauth: self.require_reauth,
        }
    }
}
