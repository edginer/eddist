use anyhow::Context;
use aws_sdk_s3::{Client, primitives::ByteStream};
use eddist_core::domain::authed_token_backup::{AUTHED_TOKENS_S3_PREFIX, AuthedTokenBackup};
use eddist_entity::authed_token;
use sea_orm::{DatabaseConnection, EntityTrait};
use uuid::Uuid;

pub async fn backup_token(
    db: &DatabaseConnection,
    client: &Client,
    bucket_name: &str,
    token_id: Uuid,
) -> anyhow::Result<()> {
    let token = authed_token::Entity::find_by_id(token_id)
        .one(db)
        .await?
        .context("authed token not found")?;
    let backup = AuthedTokenBackup {
        id: token.id,
        token: token.token,
        origin_ip: token.origin_ip,
        reduced_origin_ip: token.reduced_origin_ip,
        asn_num: token.asn_num,
        writing_ua: token.writing_ua,
        authed_ua: token.authed_ua,
        auth_code: Some(token.auth_code),
        created_at: token.created_at.naive_utc(),
        authed_at: token.authed_at.map(|dt| dt.naive_utc()),
        last_wrote_at: token.last_wrote_at.map(|dt| dt.naive_utc()),
        additional_info: token.additional_info,
        author_id_seed: token.author_id_seed,
    };

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
