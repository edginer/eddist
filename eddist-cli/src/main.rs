use anyhow::Result;
use clap::{Parser, Subcommand};
use eddist_core::domain::authed_token_backup::{AUTHED_TOKENS_S3_PREFIX, AuthedTokenBackup};
use futures::StreamExt;
use s3::creds::Credentials;
use sha2::Digest;
use std::{collections::HashSet, env, sync::Arc};
use uuid::Uuid;

const CONCURRENCY: usize = 16;

#[derive(Parser)]
#[command(name = "eddist-cli")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage authed tokens
    AuthedTokens {
        #[command(subcommand)]
        command: AuthedTokensCommand,
    },
}

#[derive(Subcommand)]
enum AuthedTokensCommand {
    /// Backup valid tokens from MySQL to S3
    Backup,
    /// Restore tokens from S3 into MySQL
    Recover,
    /// Show differences between DB and S3
    Validate,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let cli = Cli::parse();
    match cli.command {
        Commands::AuthedTokens { command } => match command {
            AuthedTokensCommand::Backup => backup().await,
            AuthedTokensCommand::Recover => recover().await,
            AuthedTokensCommand::Validate => validate().await,
        },
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

    let rows = sqlx::query_as!(
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
            additional_info AS "additional_info: serde_json::Value"
        FROM authed_tokens WHERE validity = 1"#
    )
    .fetch_all(&pool)
    .await?;

    let total = rows.len();
    println!("Backing up {total} valid tokens...");

    let results = futures::stream::iter(rows)
        .map(|token| {
            let bucket = bucket.clone();
            async move {
                let bytes = serde_json::to_vec(&token)?;
                bucket
                    .put_object(
                        format!("{AUTHED_TOKENS_S3_PREFIX}/{}.json", token.id),
                        &bytes,
                    )
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

async fn validate() -> Result<()> {
    let pool = sqlx::MySqlPool::connect(&env::var("DATABASE_URL")?).await?;
    let bucket = make_bucket()?;

    let db_ids =
        sqlx::query_scalar!(r#"SELECT id AS "id!: Uuid" FROM authed_tokens WHERE validity = 1"#)
            .fetch_all(&pool)
            .await?
            .into_iter()
            .collect::<HashSet<_>>();

    let prefix = format!("{AUTHED_TOKENS_S3_PREFIX}/");
    let s3_ids = bucket
        .list(prefix.clone(), None)
        .await?
        .into_iter()
        .flat_map(|page| page.contents)
        .filter_map(|obj| {
            let name = obj.key.strip_prefix(&prefix)?.strip_suffix(".json")?;
            Uuid::parse_str(name).ok()
        })
        .collect::<HashSet<_>>();

    println!("DB valid tokens: {}", db_ids.len());
    println!("S3 objects:      {}", s3_ids.len());

    let mut missing_from_s3 = db_ids.difference(&s3_ids).collect::<Vec<_>>();
    let mut orphaned_in_s3 = s3_ids.difference(&db_ids).collect::<Vec<_>>();
    missing_from_s3.sort();
    orphaned_in_s3.sort();

    if missing_from_s3.is_empty() && orphaned_in_s3.is_empty() {
        println!("No differences found.");
    } else {
        if !missing_from_s3.is_empty() {
            println!("\nMissing from S3 ({}):", missing_from_s3.len());
            for id in &missing_from_s3 {
                println!("  {id}");
            }
        }
        if !orphaned_in_s3.is_empty() {
            println!(
                "\nIn S3 but not in DB or invalidated ({}):",
                orphaned_in_s3.len()
            );
            for id in &orphaned_in_s3 {
                println!("  {id}");
            }
        }
    }

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
                .bind(token.id.as_bytes().to_vec())
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
