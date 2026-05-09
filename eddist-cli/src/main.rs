use anyhow::Result;
use aws_sdk_s3::{
    Client,
    config::{Credentials, Region},
    primitives::ByteStream,
};
use clap::{Parser, Subcommand};
use eddist_core::domain::authed_token_backup::{AUTHED_TOKENS_S3_PREFIX, AuthedTokenBackup};
use futures::StreamExt;
use std::{collections::HashSet, env};
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

fn make_s3_client() -> Result<(Client, String)> {
    let account_id = env::var("R2_ACCOUNT_ID")?;
    let bucket_name = env::var("S3_BUCKET_NAME")?;
    let endpoint = format!("https://{}.r2.cloudflarestorage.com", account_id.trim());
    let creds = Credentials::new(
        env::var("S3_ACCESS_KEY")?.trim(),
        env::var("S3_ACCESS_SECRET_KEY")?.trim(),
        None,
        None,
        "custom",
    );
    let config = aws_sdk_s3::Config::builder()
        .credentials_provider(creds)
        .region(Region::new("auto"))
        .endpoint_url(endpoint)
        .build();
    Ok((Client::from_conf(config), bucket_name.trim().to_string()))
}

async fn backup() -> Result<()> {
    let pool = sqlx::MySqlPool::connect(&env::var("DATABASE_URL")?).await?;
    let (client, bucket_name) = make_s3_client()?;

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
            additional_info AS "additional_info: serde_json::Value",
            author_id_seed AS "author_id_seed!: Vec<u8>"
        FROM authed_tokens WHERE validity = 1"#
    )
    .fetch_all(&pool)
    .await?;

    let total = rows.len();
    println!("Backing up {total} valid tokens...");

    let results = futures::stream::iter(rows)
        .map(|token| {
            let client = client.clone();
            let bucket_name = bucket_name.clone();
            async move {
                let bytes = serde_json::to_vec(&token)?;
                client
                    .put_object()
                    .bucket(&bucket_name)
                    .key(format!("{AUTHED_TOKENS_S3_PREFIX}/{}.json", token.id))
                    .body(ByteStream::from(bytes))
                    .send()
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
    let (client, bucket_name) = make_s3_client()?;

    let db_ids =
        sqlx::query_scalar!(r#"SELECT id AS "id!: Uuid" FROM authed_tokens WHERE validity = 1"#)
            .fetch_all(&pool)
            .await?
            .into_iter()
            .collect::<HashSet<_>>();

    let prefix = format!("{AUTHED_TOKENS_S3_PREFIX}/");
    let mut pages = client
        .list_objects_v2()
        .bucket(&bucket_name)
        .prefix(&prefix)
        .into_paginator()
        .send();
    let mut s3_ids = HashSet::new();
    while let Some(page) = pages.next().await {
        for obj in page?.contents.unwrap_or_default() {
            if let Some(key) = obj.key
                && let Some(name) = key
                    .strip_prefix(&prefix)
                    .and_then(|n| n.strip_suffix(".json"))
                && let Ok(id) = Uuid::parse_str(name)
            {
                s3_ids.insert(id);
            }
        }
    }

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
    let (client, bucket_name) = make_s3_client()?;

    let mut pages = client
        .list_objects_v2()
        .bucket(&bucket_name)
        .prefix(format!("{AUTHED_TOKENS_S3_PREFIX}/"))
        .into_paginator()
        .send();
    let mut keys = Vec::new();
    while let Some(page) = pages.next().await {
        for obj in page?.contents.unwrap_or_default() {
            if let Some(key) = obj.key {
                keys.push(key);
            }
        }
    }

    let total = keys.len();
    println!("Recovering {total} tokens from S3...");

    let results = futures::stream::iter(keys)
        .map(|key| {
            let client = client.clone();
            let bucket_name = bucket_name.clone();
            let pool = pool.clone();
            async move {
                let output = client
                    .get_object()
                    .bucket(&bucket_name)
                    .key(&key)
                    .send()
                    .await?;
                let data = output.body.collect().await?.into_bytes();
                let token: AuthedTokenBackup = serde_json::from_slice(&data)?;

                let auth_code = token.auth_code.as_deref().unwrap_or("000000");

                let result = sqlx::query!(
                    r#"INSERT IGNORE INTO authed_tokens
                       (id, token, origin_ip, reduced_origin_ip, asn_num, writing_ua, authed_ua,
                        auth_code, created_at, authed_at, validity, last_wrote_at,
                        author_id_seed, additional_info)
                       VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, ?, ?, ?)"#,
                    token.id.as_bytes().as_ref(),
                    token.token,
                    token.origin_ip,
                    token.reduced_origin_ip,
                    token.asn_num,
                    token.writing_ua,
                    token.authed_ua,
                    auth_code,
                    token.created_at,
                    token.authed_at,
                    token.last_wrote_at,
                    token.author_id_seed,
                    token.additional_info,
                )
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
