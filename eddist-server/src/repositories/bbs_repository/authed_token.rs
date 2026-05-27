use chrono::{DateTime, NaiveDateTime, Utc};
use eddist_core::domain::ip_addr::{IpAddr, ReducedIpAddr};
use sqlx::query;
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
    async fn update_authed_token_id_seed<'a>(
        &'a self,
        token_id: Uuid,
        author_id_seed: Vec<u8>,
        tx: sqlx::Transaction<'a, sqlx::MySql>,
    ) -> anyhow::Result<sqlx::Transaction<'a, sqlx::MySql>>;
    async fn clear_require_reauth(&self, id: Uuid) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
impl AuthedTokenRepository for BbsRepositoryImpl {
    async fn get_authed_token(&self, token: &str) -> anyhow::Result<Option<AuthedToken>> {
        let row = sqlx::query_as!(
            SelectionAuthedToken,
            r#"SELECT
                id AS "id: Uuid",
                token,
                origin_ip,
                reduced_origin_ip,
                asn_num,
                writing_ua,
                authed_ua,
                auth_code,
                created_at,
                authed_at,
                validity AS "validity: bool",
                last_wrote_at,
                author_id_seed,
                require_user_registration AS "require_user_registration: bool",
                registered_user_id AS "registered_user_id?: Uuid",
                require_reauth AS "require_reauth: bool"
            FROM authed_tokens WHERE token = ?"#,
            token
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(SelectionAuthedToken::into_authed_token))
    }

    async fn get_authed_token_by_id(&self, id: Uuid) -> anyhow::Result<Option<AuthedToken>> {
        let row = sqlx::query_as!(
            SelectionAuthedToken,
            r#"SELECT
                id AS "id: Uuid",
                token,
                origin_ip,
                reduced_origin_ip,
                asn_num,
                writing_ua,
                authed_ua,
                auth_code,
                created_at,
                authed_at,
                validity AS "validity: bool",
                last_wrote_at,
                author_id_seed,
                require_user_registration AS "require_user_registration: bool",
                registered_user_id AS "registered_user_id?: Uuid",
                require_reauth AS "require_reauth: bool"
            FROM authed_tokens WHERE id = ?"#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(SelectionAuthedToken::into_authed_token))
    }

    async fn get_authed_token_by_origin_ip_and_auth_code(
        &self,
        reduced_ip: &str,
        auth_code: &str,
    ) -> anyhow::Result<Option<AuthedToken>> {
        let row = sqlx::query_as!(
            SelectionAuthedToken,
            r#"SELECT
                id AS "id: Uuid",
                token,
                origin_ip,
                reduced_origin_ip,
                asn_num,
                writing_ua,
                authed_ua,
                auth_code,
                created_at,
                authed_at,
                validity AS "validity: bool",
                last_wrote_at,
                author_id_seed,
                require_user_registration AS "require_user_registration: bool",
                registered_user_id AS "registered_user_id?: Uuid",
                require_reauth AS "require_reauth: bool"
            FROM authed_tokens WHERE reduced_origin_ip = ? AND auth_code = ?"#,
            reduced_ip,
            auth_code
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(SelectionAuthedToken::into_authed_token))
    }

    async fn get_unauthed_authed_token_by_auth_code(
        &self,
        auth_code: &str,
    ) -> anyhow::Result<Vec<AuthedToken>> {
        let rows = sqlx::query_as!(
            SelectionAuthedToken,
            r#"SELECT
                id AS "id: Uuid",
                token,
                origin_ip,
                reduced_origin_ip,
                asn_num,
                writing_ua,
                authed_ua,
                auth_code,
                created_at,
                authed_at,
                validity AS "validity: bool",
                last_wrote_at,
                author_id_seed,
                require_user_registration AS "require_user_registration: bool",
                registered_user_id AS "registered_user_id?: Uuid",
                require_reauth AS "require_reauth: bool"
            FROM authed_tokens WHERE auth_code = ? AND validity = false"#,
            auth_code
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(SelectionAuthedToken::into_authed_token)
            .collect())
    }

    async fn create_authed_token(&self, authed_token: CreatingAuthedToken) -> anyhow::Result<()> {
        let ip_addr = authed_token.origin_ip.to_string();
        let reduced_ip = ReducedIpAddr::from(authed_token.origin_ip).to_string();

        query!(
            "INSERT INTO authed_tokens
                (
                    id,
                    token,
                    origin_ip,
                    reduced_origin_ip,
                    asn_num,
                    writing_ua,
                    auth_code,
                    created_at,
                    validity,
                    author_id_seed,
                    require_user_registration
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            authed_token.id,
            authed_token.token,
            ip_addr,
            reduced_ip,
            authed_token.asn_num,
            authed_token.writing_ua,
            authed_token.auth_code,
            authed_token.created_at,
            false,
            authed_token.author_id_seed,
            authed_token.require_user_registration,
        )
        .execute(&self.pool)
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
        let additional_info_json = additional_info.and_then(|v| serde_json::to_string(&v).ok());

        query!(
            "UPDATE authed_tokens SET validity = ?, authed_ua = ?, authed_at = ?, additional_info = ? WHERE token = ?",
            true,
            authed_ua,
            authed_time,
            additional_info_json,
            token,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update_authed_token_last_wrote(
        &self,
        token_id: Uuid,
        last_wrote: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        query!(
            "UPDATE authed_tokens SET last_wrote_at = ? WHERE id = ?",
            last_wrote,
            token_id,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn revoke_authed_token(&self, token: &str) -> anyhow::Result<()> {
        query!(
            "UPDATE authed_tokens SET validity = ? WHERE token = ?",
            false,
            token
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete_authed_token(&self, token: &str) -> anyhow::Result<()> {
        query!("DELETE FROM authed_tokens WHERE token = ?", token)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn update_authed_token_id_seed<'a>(
        &'a self,
        token_id: Uuid,
        author_id_seed: Vec<u8>,
        mut tx: sqlx::Transaction<'a, sqlx::MySql>,
    ) -> anyhow::Result<sqlx::Transaction<'a, sqlx::MySql>> {
        query!(
            "UPDATE authed_tokens SET author_id_seed = ? WHERE id = ?",
            author_id_seed,
            token_id,
        )
        .execute(&mut *tx)
        .await?;

        Ok(tx)
    }

    async fn clear_require_reauth(&self, id: Uuid) -> anyhow::Result<()> {
        query!(
            "UPDATE authed_tokens SET require_reauth = 0 WHERE id = ?",
            id
        )
        .execute(&self.pool)
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

#[derive(Debug)]
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
