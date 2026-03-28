use anyhow::Result;
use eddist_core::domain::authed_token_backup::{AUTHED_TOKENS_S3_PREFIX, AuthedTokenBackup};
use futures::StreamExt;
use s3::creds::Credentials;
use sha2::Digest;
use sqlx::Row;
use std::{env, sync::Arc};
use uuid::Uuid;

const CONCURRENCY: usize = 16;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let command = std::env::args().nth(1);
    match command.as_deref() {
        Some("backup") => backup().await,
        Some("recover") => recover().await,
        _ => {
            eprintln!("Usage: eddist-cli <backup|recover>");
            std::process::exit(1);
        }
    }
}

fn make_bucket() -> Result<Arc<s3::Bucket>> {
    Ok(Arc::from(s3::Bucket::new(
        env::var("S3_BUCKET_NAME")?.trim(),
        s3::Region::R2 {
            account_id: env::var("R2_ACCOUNT_ID")?.trim().to_string(),
        },
        Credentials::new(
            Some(env::var("S3_ACCESS_KEY")?.trim()),
            Some(env::var("S3_ACCESS_SECRET_KEY")?.trim()),
            None,
            None,
            None,
        )?,
    )?))
}

async fn backup() -> Result<()> {
    let pool = sqlx::MySqlPool::connect(&env::var("DATABASE_URL")?).await?;
    let bucket = make_bucket()?;

    let rows = sqlx::query(
        "SELECT id, token, origin_ip, reduced_origin_ip, asn_num, writing_ua, authed_ua, \
         auth_code, created_at, authed_at, last_wrote_at, additional_info \
         FROM authed_tokens WHERE validity = 1",
    )
    .fetch_all(&pool)
    .await?;

    let total = rows.len();
    println!("Backing up {total} valid tokens...");

    let results = futures::stream::iter(rows)
        .map(|row| {
            let bucket = bucket.clone();
            async move {
                let id_bytes: Vec<u8> = row.try_get("id")?;
                let uuid = Uuid::from_slice(&id_bytes)?;

                let token = AuthedTokenBackup {
                    id: uuid.to_string(),
                    token: row.try_get("token")?,
                    origin_ip: row.try_get("origin_ip")?,
                    reduced_origin_ip: row.try_get("reduced_origin_ip")?,
                    asn_num: row.try_get("asn_num")?,
                    writing_ua: row.try_get("writing_ua")?,
                    authed_ua: row.try_get("authed_ua")?,
                    auth_code: row.try_get("auth_code")?,
                    created_at: row.try_get("created_at")?,
                    authed_at: row.try_get("authed_at")?,
                    last_wrote_at: row.try_get("last_wrote_at")?,
                    additional_info: row
                        .try_get::<Option<serde_json::Value>, _>("additional_info")
                        .unwrap_or_default(),
                };

                let bytes = serde_json::to_vec(&token)?;
                bucket
                    .put_object(format!("{AUTHED_TOKENS_S3_PREFIX}/{uuid}.json"), &bytes)
                    .await?;

                anyhow::Ok(())
            }
        })
        .buffer_unordered(CONCURRENCY)
        .collect::<Vec<_>>()
        .await;

    let errors = results.iter().filter(|r| r.is_err()).count();
    if errors > 0 {
        eprintln!("{errors} tokens failed to backup");
    }
    println!("Done. Backed up {}/{total} tokens.", total - errors);
    Ok(())
}

async fn recover() -> Result<()> {
    let pool = sqlx::MySqlPool::connect(&env::var("DATABASE_URL")?).await?;
    let bucket = make_bucket()?;

    let keys = bucket
        .list(format!("{AUTHED_TOKENS_S3_PREFIX}/"), None)
        .await?
        .into_iter()
        .flat_map(|page| page.contents)
        .map(|obj| obj.key)
        .collect::<Vec<_>>();

    let total = keys.len();
    println!("Recovering {total} tokens from S3...");

    let results = futures::stream::iter(keys)
        .map(|key| {
            let bucket = bucket.clone();
            let pool = pool.clone();
            async move {
                let data = bucket.get_object(&key).await?;
                let token: AuthedTokenBackup = serde_json::from_slice(data.bytes())?;

                let uuid = Uuid::parse_str(&token.id)?;
                // author_id_seed is not stored in the backup; recompute from reduced_origin_ip
                // to match the formula in the add_author_id_generation_col migration.
                let author_id_seed = sha2::Sha512::digest(token.reduced_origin_ip.as_bytes());
                let auth_code = token.auth_code.as_deref().unwrap_or("000000");

                let result = sqlx::query(
                    "INSERT IGNORE INTO authed_tokens \
                     (id, token, origin_ip, reduced_origin_ip, asn_num, writing_ua, authed_ua, \
                      auth_code, created_at, authed_at, validity, last_wrote_at, \
                      author_id_seed, additional_info) \
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, ?, ?, ?)",
                )
                .bind(uuid.as_bytes().to_vec())
                .bind(&token.token)
                .bind(&token.origin_ip)
                .bind(&token.reduced_origin_ip)
                .bind(token.asn_num)
                .bind(&token.writing_ua)
                .bind(&token.authed_ua)
                .bind(auth_code)
                .bind(token.created_at)
                .bind(token.authed_at)
                .bind(token.last_wrote_at)
                .bind(author_id_seed.as_slice())
                .bind(&token.additional_info)
                .execute(&pool)
                .await?;

                anyhow::Ok(result.rows_affected() > 0)
            }
        })
        .buffer_unordered(CONCURRENCY)
        .collect::<Vec<_>>()
        .await;

    let inserted = results
        .iter()
        .filter(|r| r.as_ref().is_ok_and(|b| *b))
        .count();
    let skipped = results
        .iter()
        .filter(|r| r.as_ref().is_ok_and(|b| !*b))
        .count();
    let errors = results.iter().filter(|r| r.is_err()).count();
    if errors > 0 {
        eprintln!("{errors} tokens failed to recover");
    }
    println!("Done. Inserted {inserted}, skipped {skipped} already-existing tokens.");
    Ok(())
}
