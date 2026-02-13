use chrono::NaiveDateTime;
use sqlx::{query, query_as, FromRow, MySql, QueryBuilder, Row};
use uuid::Uuid;

use crate::models::AuthedToken;

#[derive(Debug, FromRow)]
struct AuthedTokenRow {
    pub id: Vec<u8>,
    pub token: String,
    pub origin_ip: String,
    pub reduced_origin_ip: String,
    pub asn_num: i32,
    pub writing_ua: String,
    pub authed_ua: Option<String>,
    pub created_at: NaiveDateTime,
    pub authed_at: Option<NaiveDateTime>,
    pub validity: bool,
    pub last_wrote_at: Option<NaiveDateTime>,
    pub additional_info: Option<serde_json::Value>,
}

impl From<AuthedTokenRow> for AuthedToken {
    fn from(row: AuthedTokenRow) -> Self {
        Self {
            id: Uuid::from_slice(&row.id).unwrap(),
            token: row.token,
            origin_ip: row.origin_ip,
            reduced_origin_ip: row.reduced_origin_ip,
            asn_num: row.asn_num,
            writing_ua: row.writing_ua,
            authed_ua: row.authed_ua,
            created_at: row.created_at,
            authed_at: row.authed_at,
            validity: row.validity,
            last_wrote_at: row.last_wrote_at,
            additional_info: row.additional_info,
        }
    }
}

#[async_trait::async_trait]
pub trait AuthedTokenRepository: Send + Sync {
    async fn get_authed_token(&self, id: Uuid) -> anyhow::Result<AuthedToken>;
    async fn delete_authed_token(&self, id: Uuid) -> anyhow::Result<()>;
    async fn delete_authed_token_by_origin_ip(&self, id: Uuid) -> anyhow::Result<()>;
    async fn list_authed_tokens(
        &self,
        offset: u64,
        limit: u32,
        origin_ip: Option<&str>,
        writing_ua: Option<&str>,
        authed_ua: Option<&str>,
        asn_num: Option<i32>,
        validity: Option<bool>,
        sort_column: &str,
        sort_asc: bool,
    ) -> anyhow::Result<(Vec<AuthedToken>, u64)>;
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
                asn_num,
                writing_ua,
                authed_ua,
                created_at,
                authed_at,
                validity AS "validity!: bool",
                last_wrote_at,
                additional_info AS "additional_info: serde_json::Value"
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
                origin_ip IN (
                    SELECT origin_ip FROM (
                        SELECT
                            origin_ip
                        FROM
                            authed_tokens
                        WHERE
                            id = ?
                    ) tmp
                )
        "#,
            id.as_bytes().to_vec(),
        );

        query.execute(&self.0).await?;

        Ok(())
    }

    async fn list_authed_tokens(
        &self,
        offset: u64,
        limit: u32,
        origin_ip: Option<&str>,
        writing_ua: Option<&str>,
        authed_ua: Option<&str>,
        asn_num: Option<i32>,
        validity: Option<bool>,
        sort_column: &str,
        sort_asc: bool,
    ) -> anyhow::Result<(Vec<AuthedToken>, u64)> {
        let mut count_builder: QueryBuilder<MySql> =
            QueryBuilder::new("SELECT COUNT(*) as cnt FROM authed_tokens WHERE 1=1");
        let mut data_builder: QueryBuilder<MySql> = QueryBuilder::new(
            "SELECT id, token, origin_ip, reduced_origin_ip, asn_num, writing_ua, authed_ua, created_at, authed_at, validity, last_wrote_at, additional_info FROM authed_tokens WHERE 1=1",
        );

        if let Some(ip) = origin_ip {
            count_builder.push(" AND origin_ip = ");
            count_builder.push_bind(ip.to_string());
            data_builder.push(" AND origin_ip = ");
            data_builder.push_bind(ip.to_string());
        }
        if let Some(ua) = writing_ua {
            count_builder.push(" AND writing_ua LIKE ");
            count_builder.push_bind(format!("%{ua}%"));
            data_builder.push(" AND writing_ua LIKE ");
            data_builder.push_bind(format!("%{ua}%"));
        }
        if let Some(ua) = authed_ua {
            count_builder.push(" AND authed_ua LIKE ");
            count_builder.push_bind(format!("%{ua}%"));
            data_builder.push(" AND authed_ua LIKE ");
            data_builder.push_bind(format!("%{ua}%"));
        }
        if let Some(asn) = asn_num {
            count_builder.push(" AND asn_num = ");
            count_builder.push_bind(asn);
            data_builder.push(" AND asn_num = ");
            data_builder.push_bind(asn);
        }
        if let Some(v) = validity {
            count_builder.push(" AND validity = ");
            count_builder.push_bind(v);
            data_builder.push(" AND validity = ");
            data_builder.push_bind(v);
        }

        let direction = if sort_asc { " ASC" } else { " DESC" };
        data_builder.push(format!(" ORDER BY {sort_column}{direction}"));
        data_builder.push(" LIMIT ");
        data_builder.push_bind(limit as i64);
        data_builder.push(" OFFSET ");
        data_builder.push_bind(offset as i64);

        let count_row = count_builder.build().fetch_one(&self.0).await?;
        let total: i64 = count_row.get("cnt");

        let rows = data_builder
            .build_query_as::<AuthedTokenRow>()
            .fetch_all(&self.0)
            .await?;

        let tokens = rows.into_iter().map(AuthedToken::from).collect();

        Ok((tokens, total as u64))
    }
}
