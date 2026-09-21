#[cfg(not(feature = "backend-postgres"))]
use crate::entity::authed_token;
use anyhow::Result;
use aws_sdk_s3::{
    Client,
    config::{Credentials, Region},
    primitives::ByteStream,
};
use clap::{Parser, Subcommand};
use eddist_core::domain::authed_token_backup::{AUTHED_TOKENS_S3_PREFIX, AuthedTokenBackup};
use futures::StreamExt;
#[cfg(not(feature = "backend-postgres"))]
use sea_orm::{
    ActiveValue::Set, ColumnTrait, ConnectOptions, Database, DatabaseConnection, EntityTrait,
    QueryFilter, QuerySelect, TryInsertResult,
};
use std::{collections::HashSet, env};
use uuid::Uuid;

#[cfg(not(feature = "backend-postgres"))]
mod entity;
mod migrate;

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
    /// Migrate data from MySQL to PostgreSQL (excludes archived_responses and archived_threads)
    Migrate {
        /// MySQL connection URL (defaults to DATABASE_URL env var)
        #[arg(long)]
        mysql_url: Option<String>,
        /// PostgreSQL connection URL (defaults to PG_DATABASE_URL env var)
        #[arg(long)]
        pg_url: Option<String>,
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
        Commands::Migrate { mysql_url, pg_url } => {
            let mysql_url = mysql_url
                .or_else(|| env::var("DATABASE_URL").ok())
                .expect("provide --mysql-url or set DATABASE_URL / MYSQL_URL");
            let pg_url = pg_url
                .or_else(|| env::var("PG_DATABASE_URL").ok())
                .expect("provide --pg-url or set PG_DATABASE_URL");
            migrate::run(&mysql_url, &pg_url).await
        }
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
        .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
        .credentials_provider(creds)
        .region(Region::new("auto"))
        .endpoint_url(endpoint)
        .build();
    Ok((Client::from_conf(config), bucket_name.trim().to_string()))
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
async fn backup() -> Result<()> {
    let pool = sqlx::PgPool::connect(&env::var("DATABASE_URL")?).await?;
    let (client, bucket_name) = make_s3_client()?;

    let rows = sqlx::query_as!(
        AuthedTokenBackupPg,
        r#"SELECT id, token, origin_ip, reduced_origin_ip, asn_num, writing_ua, authed_ua,
                  auth_code, created_at, authed_at, last_wrote_at, additional_info, author_id_seed
           FROM authed_tokens WHERE validity = TRUE"#,
    )
    .fetch_all(&pool)
    .await?;

    let total = rows.len();
    println!("Backing up {total} valid tokens...");

    let results = futures::stream::iter(rows)
        .map(|row| {
            let client = client.clone();
            let bucket_name = bucket_name.clone();
            let token = AuthedTokenBackup::from(row);
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

#[cfg(feature = "backend-postgres")]
async fn validate() -> Result<()> {
    let pool = sqlx::PgPool::connect(&env::var("DATABASE_URL")?).await?;
    let (client, bucket_name) = make_s3_client()?;

    let db_ids =
        sqlx::query_scalar!(r#"SELECT id AS "id!: Uuid" FROM authed_tokens WHERE validity = TRUE"#)
            .fetch_all(&pool)
            .await?;
    let db_ids = db_ids.into_iter().collect::<HashSet<_>>();

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

#[cfg(feature = "backend-postgres")]
async fn recover() -> Result<()> {
    let pool = sqlx::PgPool::connect(&env::var("DATABASE_URL")?).await?;
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
                let created_at = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                    token.created_at,
                    chrono::Utc,
                );
                let authed_at = token.authed_at.map(|dt| {
                    chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc)
                });
                let last_wrote_at = token.last_wrote_at.map(|dt| {
                    chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc)
                });

                let result = sqlx::query!(
                    r#"INSERT INTO authed_tokens
                       (id, token, origin_ip, reduced_origin_ip, asn_num, writing_ua, authed_ua,
                        auth_code, created_at, authed_at, validity, last_wrote_at,
                        author_id_seed, additional_info)
                       VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, TRUE, $11, $12, $13)
                       ON CONFLICT DO NOTHING"#,
                    token.id,
                    &token.token,
                    &token.origin_ip,
                    &token.reduced_origin_ip,
                    token.asn_num,
                    &token.writing_ua,
                    token.authed_ua,
                    auth_code,
                    created_at,
                    authed_at,
                    last_wrote_at,
                    &token.author_id_seed,
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

#[cfg(not(feature = "backend-postgres"))]
async fn connect_database() -> Result<DatabaseConnection> {
    let mut options = ConnectOptions::new(env::var("DATABASE_URL")?);
    options.sqlx_logging(false);
    Ok(Database::connect(options).await?)
}

#[cfg(not(feature = "backend-postgres"))]
impl From<authed_token::Model> for AuthedTokenBackup {
    fn from(token: authed_token::Model) -> Self {
        Self {
            id: token.id,
            token: token.token,
            origin_ip: token.origin_ip,
            reduced_origin_ip: token.reduced_origin_ip,
            asn_num: token.asn_num,
            writing_ua: token.writing_ua,
            authed_ua: token.authed_ua,
            auth_code: Some(token.auth_code),
            created_at: token.created_at,
            authed_at: token.authed_at,
            last_wrote_at: token.last_wrote_at,
            additional_info: token.additional_info,
            author_id_seed: token.author_id_seed,
        }
    }
}

#[cfg(not(feature = "backend-postgres"))]
async fn backup() -> Result<()> {
    let db = connect_database().await?;
    let (client, bucket_name) = make_s3_client()?;

    let rows = authed_token::Entity::find()
        .filter(authed_token::Column::Validity.eq(true))
        .all(&db)
        .await?;
    let rows = rows
        .into_iter()
        .map(AuthedTokenBackup::from)
        .collect::<Vec<_>>();

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

#[cfg(not(feature = "backend-postgres"))]
async fn validate() -> Result<()> {
    let db = connect_database().await?;
    let (client, bucket_name) = make_s3_client()?;

    let db_ids = authed_token::Entity::find()
        .select_only()
        .column(authed_token::Column::Id)
        .filter(authed_token::Column::Validity.eq(true))
        .into_tuple::<Uuid>()
        .all(&db)
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

#[cfg(not(feature = "backend-postgres"))]
async fn recover() -> Result<()> {
    let db = connect_database().await?;
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
            let db = db.clone();
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

                let result = authed_token::Entity::insert(authed_token::ActiveModel {
                    id: Set(token.id),
                    token: Set(token.token),
                    origin_ip: Set(token.origin_ip),
                    reduced_origin_ip: Set(token.reduced_origin_ip),
                    writing_ua: Set(token.writing_ua),
                    authed_ua: Set(token.authed_ua),
                    auth_code: Set(auth_code.to_string()),
                    created_at: Set(token.created_at),
                    authed_at: Set(token.authed_at),
                    validity: Set(true),
                    last_wrote_at: Set(token.last_wrote_at),
                    asn_num: Set(token.asn_num),
                    additional_info: Set(token.additional_info),
                    author_id_seed: Set(token.author_id_seed),
                    ..Default::default()
                })
                .on_conflict_do_nothing_on([authed_token::Column::Id])
                .exec(&db)
                .await?;

                anyhow::Ok(matches!(result, TryInsertResult::Inserted(_)))
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
