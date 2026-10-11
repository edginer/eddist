#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::{convert::Infallible, time::Duration};

use axum::{
    ServiceExt as AxumServiceExt, body::Body, extract::Request as AxumRequest, response::Response,
};
use eddist::{
    app::{AppDependencies, build_app},
    config::ServerConfig,
};
use eddist_core::{tracing::init_tracing, utils::is_prod};
use hyper::{server::conn::http1, service::service_fn};
use hyper_util::rt::{TokioIo, TokioTimer};
use metrics::describe_counter;
use sqlx::mysql::MySqlPoolOptions;
use tokio::net::TcpListener;
use tower::Layer;
use tower_http::normalize_path::NormalizePathLayer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if !is_prod() {
        dotenvy::dotenv()?;
    }

    init_tracing();

    let config = ServerConfig::from_env()?;

    let client = redis::Client::open(config.redis_url.as_str())?;
    let conn_mgr = client.get_connection_manager().await?;

    let pool = MySqlPoolOptions::new()
        .after_connect(|conn, _| {
            use sqlx::Executor;
            Box::pin(async move {
                // Set transaction isolation level to `READ-COMMITTED`
                conn.execute("SET SESSION TRANSACTION ISOLATION LEVEL READ COMMITTED")
                    .await
                    .unwrap();
                log::info!("Set transaction isolation level to `READ-COMMITTED`");

                // Set TIME_TRUNCATE_FRACTIONAL mode to match chrono's %3f truncation behavior
                conn.execute(
                    "SET SESSION sql_mode = CONCAT(@@sql_mode, ',TIME_TRUNCATE_FRACTIONAL')",
                )
                .await
                .unwrap();
                log::info!("Set TIME_TRUNCATE_FRACTIONAL mode");
                Ok(())
            })
        })
        .max_connections(8)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.database_url)
        .await?;

    let s3_creds = aws_sdk_s3::config::Credentials::new(
        config.s3.access_key,
        config.s3.secret_access_key,
        None,
        None,
        "custom",
    );
    let s3_config = aws_sdk_s3::Config::builder()
        .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
        .credentials_provider(s3_creds)
        .region(aws_sdk_s3::config::Region::new("auto"))
        .endpoint_url(config.s3.endpoint)
        .build();

    let app = build_app(
        config.app,
        AppDependencies {
            pool,
            redis_conn: conn_mgr,
            s3_client: aws_sdk_s3::Client::from_conf(s3_config),
            s3_bucket_name: config.s3.bucket_name,
        },
    )
    .await?;

    log::info!("Start application server with 0.0.0.0:{}", config.port);

    describe_counter!("token_request", "token request count from bbs.cgi by state");
    describe_counter!(
        "auth_code_request",
        "auth code request count from auth-code endpoint"
    );
    describe_counter!("auth_code_failure", "auth code failure count by reason");
    describe_counter!("auth_code_success", "auth code success count");
    describe_counter!("response_creation", "response creation count if success");
    describe_counter!("thread_creation", "thread creation count if success");
    describe_counter!(
        "dat_retrieval",
        "dat file retrieval count by source (cache or db)"
    );
    describe_counter!(
        "openai_moderation_requests",
        "OpenAI moderation request count by result (success/error)"
    );
    describe_counter!(
        "openai_moderation_api_calls",
        "individual HTTP calls to OpenAI moderation API including retries"
    );
    describe_counter!(
        "openai_moderation_retries",
        "OpenAI moderation retry count (excludes first attempt)"
    );

    let listener = TcpListener::bind(("0.0.0.0", config.port)).await?;

    let router = NormalizePathLayer::trim_trailing_slash().layer(app.router.clone());
    let router = AxumServiceExt::<AxumRequest>::into_make_service(router);
    axum::serve(listener, router)
        .with_graceful_shutdown(graceful_shutdown_http())
        .await
        .unwrap();

    if app.flush_stats().await.is_ok() {
        tracing::info!("Flushed stats on shutdown.");
    }

    tokio::time::sleep(Duration::from_millis(3000)).await;

    tracing::info!("Server has shut down gracufully.");
    Ok(())
}

async fn graceful_shutdown_http() {
    let listener = TcpListener::bind(("0.0.0.0", 44608)).await.unwrap();
    let (stream, _) = listener.accept().await.unwrap();

    http1::Builder::new()
        .timer(TokioTimer::new())
        .serve_connection(
            TokioIo::new(stream),
            service_fn(|_| async move {
                let response = Response::new(Body::from("Request received. Shutting down.\n"));

                Ok::<_, Infallible>(response)
            }),
        )
        .await
        .unwrap();

    tracing::info!("Server starts gracefull shutdown.");
}
