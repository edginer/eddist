#[cfg(test)]
mod integration_tests;
mod llm_moderation;
mod persistence;
mod shutdown;
mod subscriber;
mod token_backup;

use std::env;

use aws_sdk_s3::{
    Client,
    config::{Credentials, Region},
};
use eddist_core::{
    tracing::init_tracing,
    utils::{is_authed_token_backup_enabled, is_prod},
};
use sea_orm::{ConnectOptions, Database};
use tokio::join;

use subscriber::SubRepository;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if !is_prod() {
        dotenvy::dotenv().ok();
    }

    init_tracing();

    let (ctrl_c_tx, _) = tokio::sync::broadcast::channel::<()>(1);
    let ctrl_c_sub_persitence = ctrl_c_tx.subscribe();
    let ctrl_c_sub_sub = ctrl_c_tx.subscribe();

    tokio::spawn(shutdown::run_shutdown_server(ctrl_c_tx));

    let (s3_client, s3_bucket_name) = if is_authed_token_backup_enabled() {
        let bucket_name = env::var("S3_BUCKET_NAME")?;
        let account_id = env::var("R2_ACCOUNT_ID")?;
        let access_key = env::var("S3_ACCESS_KEY")?;
        let secret_key = env::var("S3_ACCESS_SECRET_KEY")?;
        let endpoint = format!("https://{}.r2.cloudflarestorage.com", account_id.trim());
        let creds = Credentials::new(access_key.trim(), secret_key.trim(), None, None, "custom");
        let config = aws_sdk_s3::Config::builder()
            .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
            .credentials_provider(creds)
            .region(Region::new("auto"))
            .endpoint_url(endpoint)
            .build();
        (
            Some(Client::from_conf(config)),
            Some(bucket_name.trim().to_string()),
        )
    } else {
        (None, None)
    };

    let mut connect_options = ConnectOptions::new(env::var("DATABASE_URL")?);
    connect_options.sqlx_logging(false);
    let db = Database::connect(connect_options).await?;

    let client = redis::Client::open(env::var("REDIS_URL").unwrap())?;
    let pubsub_conn = client.get_async_pubsub().await?;
    let conn = client.get_connection_manager().await?;

    let llm_moderation = llm_moderation::LlmModeration::default();
    // Loaded before subscribing so the first thread_created events already see
    // whether LLM moderation owns the unsafe set.
    llm_moderation.refresh_settings(&db).await;
    tokio::spawn(llm_moderation::run_loop(
        llm_moderation.clone(),
        db.clone(),
        conn.clone(),
    ));

    let mut sub_repo = subscriber::RedisSubRepository::new(
        pubsub_conn,
        conn.clone(),
        ctrl_c_sub_sub,
        s3_client,
        s3_bucket_name,
        db.clone(),
        llm_moderation,
    );

    let subscribe_handle = tokio::spawn(async move { sub_repo.subscribe().await });
    let persistence_handle = tokio::spawn(persistence::run_persistence_loop(
        conn,
        db,
        ctrl_c_sub_persitence,
    ));

    match join!(subscribe_handle, persistence_handle) {
        (Ok(_), Ok(_)) => {}
        _ => panic!(),
    }

    Ok(())
}
