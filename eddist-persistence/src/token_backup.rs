use aws_sdk_s3::{Client, primitives::ByteStream};
use eddist_core::domain::authed_token_backup::{AUTHED_TOKENS_S3_PREFIX, AuthedTokenBackup};
use uuid::Uuid;

#[cfg(not(feature = "backend-postgres"))]
pub async fn backup_token(
    pool: &sqlx::MySqlPool,
    client: &Client,
    bucket_name: &str,
    token_id: Uuid,
) -> anyhow::Result<()> {
    let backup = sqlx::query_as!(
        AuthedTokenBackup,
        r#"SELECT
            id AS "id!: Uuid",
            token,
            origin_ip,
            reduced_origin_ip,
            asn_num,
            writing_ua,
            authed_ua,
            auth_code,
            created_at,
            authed_at,
            last_wrote_at,
            additional_info AS "additional_info: serde_json::Value",
            author_id_seed AS "author_id_seed!: Vec<u8>"
        FROM authed_tokens WHERE id = ?"#,
        token_id.as_bytes().to_vec()
    )
    .fetch_one(pool)
    .await?;

    let bytes = serde_json::to_vec(&backup)?;
    client
        .put_object()
        .bucket(bucket_name)
        .key(format!("{AUTHED_TOKENS_S3_PREFIX}/{token_id}.json"))
        .body(ByteStream::from(bytes))
        .send()
        .await?;

    Ok(())
}

#[cfg(feature = "backend-postgres")]
#[derive(Debug, sqlx::FromRow)]
struct AuthedTokenBackupPg {
    pub id: Uuid,
    pub token: String,
    pub origin_ip: String,
    pub reduced_origin_ip: String,
    pub asn_num: i32,
    pub writing_ua: String,
    pub authed_ua: Option<String>,
    pub auth_code: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub authed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_wrote_at: Option<chrono::DateTime<chrono::Utc>>,
    pub additional_info: Option<serde_json::Value>,
    pub author_id_seed: Vec<u8>,
}

#[cfg(feature = "backend-postgres")]
impl From<AuthedTokenBackupPg> for AuthedTokenBackup {
    fn from(r: AuthedTokenBackupPg) -> Self {
        Self {
            id: r.id,
            token: r.token,
            origin_ip: r.origin_ip,
            reduced_origin_ip: r.reduced_origin_ip,
            asn_num: r.asn_num,
            writing_ua: r.writing_ua,
            authed_ua: r.authed_ua,
            auth_code: r.auth_code,
            created_at: r.created_at.naive_utc(),
            authed_at: r.authed_at.map(|dt| dt.naive_utc()),
            last_wrote_at: r.last_wrote_at.map(|dt| dt.naive_utc()),
            additional_info: r.additional_info,
            author_id_seed: r.author_id_seed,
        }
    }
}

#[cfg(feature = "backend-postgres")]
pub async fn backup_token(
    pool: &sqlx::PgPool,
    client: &Client,
    bucket_name: &str,
    token_id: Uuid,
) -> anyhow::Result<()> {
    let row = sqlx::query_as::<_, AuthedTokenBackupPg>(
        "SELECT id, token, origin_ip, reduced_origin_ip, asn_num, writing_ua, authed_ua,
                auth_code, created_at, authed_at, last_wrote_at, additional_info, author_id_seed
         FROM authed_tokens WHERE id = $1",
    )
    .bind(token_id)
    .fetch_one(pool)
    .await?;

    let backup = AuthedTokenBackup::from(row);
    let bytes = serde_json::to_vec(&backup)?;
    client
        .put_object()
        .bucket(bucket_name)
        .key(format!("{AUTHED_TOKENS_S3_PREFIX}/{token_id}.json"))
        .body(ByteStream::from(bytes))
        .send()
        .await?;

    Ok(())
}

pub async fn remove_token_backup(
    client: &Client,
    bucket_name: &str,
    token_id: Uuid,
) -> anyhow::Result<()> {
    client
        .delete_object()
        .bucket(bucket_name)
        .key(format!("{AUTHED_TOKENS_S3_PREFIX}/{token_id}.json"))
        .await?;
    Ok(())
}
