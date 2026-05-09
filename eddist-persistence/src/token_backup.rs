use aws_sdk_s3::{Client, primitives::ByteStream};
use eddist_core::domain::authed_token_backup::{AUTHED_TOKENS_S3_PREFIX, AuthedTokenBackup};
use uuid::Uuid;

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

pub async fn remove_token_backup(
    client: &Client,
    bucket_name: &str,
    token_id: Uuid,
) -> anyhow::Result<()> {
    client
        .delete_object()
        .bucket(bucket_name)
        .key(format!("{AUTHED_TOKENS_S3_PREFIX}/{token_id}.json"))
        .send()
        .await?;
    Ok(())
}
