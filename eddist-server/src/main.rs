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
use tokio_util::sync::CancellationToken;
use tower::Layer;
use tower_http::normalize_path::NormalizePathLayer;

const SHUTDOWN_ENDPOINT_PORT: u16 = 44608;
/// Matches the per-request `TimeoutLayer`, so only stuck connections are cut off.
const HTTP_DRAIN_DEADLINE: Duration = Duration::from_secs(10);

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
    describe_counter!(
        "background_task_panic",
        "background task panics by task name"
    );
    describe_counter!(
        "response_persistence",
        "response persistence result (db_success/db_failure/fallback_success/fallback_failure)"
    );
    describe_counter!(
        "event_publish",
        "pub/sub event publish result by event name (success/failure)"
    );
    describe_counter!(
        "token_binding",
        "authed token binding to a logged-in user result (success/failure)"
    );

    let listener = TcpListener::bind(("0.0.0.0", config.port)).await?;

    let router = NormalizePathLayer::trim_trailing_slash().layer(app.router.clone());
    let router = AxumServiceExt::<AxumRequest>::into_make_service(router);
    let shutdown = CancellationToken::new();
    tokio::spawn(watch_shutdown_signals(shutdown.clone()));
    let serve =
        axum::serve(listener, router).with_graceful_shutdown(shutdown.clone().cancelled_owned());
    tokio::select! {
        result = serve => result.unwrap(),
        () = async {
            shutdown.cancelled().await;
            tokio::time::sleep(HTTP_DRAIN_DEADLINE).await;
        } => {
            tracing::warn!(
                deadline = ?HTTP_DRAIN_DEADLINE,
                "HTTP connections did not close before the shutdown deadline; dropping them"
            );
        }
    }

    app.shutdown().await;

    tracing::info!("Server has shut down gracefully.");
    Ok(())
}

async fn watch_shutdown_signals(shutdown: CancellationToken) {
    shutdown_signal().await;
    shutdown.cancel();

    // tokio's handlers have replaced the default SIGTERM/SIGINT action of exiting.
    tokio::select! {
        _ = terminate_signal() => {}
        _ = tokio::signal::ctrl_c() => {}
    }
    tracing::warn!("Received a second shutdown signal; exiting immediately.");
    std::process::exit(1);
}

async fn shutdown_signal() {
    tokio::select! {
        _ = shutdown_endpoint() => {
            tracing::info!("Received a shutdown request on the HTTP shutdown endpoint.");
        }
        _ = terminate_signal() => {
            tracing::info!("Received SIGTERM; shutting down.");
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Received an interrupt; shutting down.");
        }
    }
}

async fn shutdown_endpoint() {
    let listener = match TcpListener::bind(("0.0.0.0", SHUTDOWN_ENDPOINT_PORT)).await {
        Ok(listener) => listener,
        Err(e) => {
            tracing::error!(
                error = %e,
                port = SHUTDOWN_ENDPOINT_PORT,
                "failed to bind the HTTP shutdown endpoint; SIGTERM/SIGINT remain available"
            );
            std::future::pending().await
        }
    };

    let stream = loop {
        match listener.accept().await {
            Ok((stream, _)) => break stream,
            Err(e) => {
                tracing::warn!(error = %e, "failed to accept on the HTTP shutdown endpoint");
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    };
    let result = http1::Builder::new()
        .timer(TokioTimer::new())
        .serve_connection(
            TokioIo::new(stream),
            service_fn(|_| async {
                let response = Response::new(Body::from("Request received. Shutting down.\n"));
                Ok::<_, Infallible>(response)
            }),
        )
        .await;
    if let Err(e) = result {
        tracing::warn!(error = %e, "shutdown endpoint connection ended with an error");
    }

    tracing::info!("Server starts graceful shutdown.");
}

#[cfg(unix)]
async fn terminate_signal() {
    use tokio::signal::unix::{SignalKind, signal};
    match signal(SignalKind::terminate()) {
        Ok(mut sigterm) => {
            sigterm.recv().await;
        }
        Err(e) => {
            tracing::error!(
                error = %e,
                "failed to listen for SIGTERM; SIGINT and the shutdown endpoint remain available"
            );
            std::future::pending().await
        }
    }
}

#[cfg(not(unix))]
async fn terminate_signal() {
    std::future::pending().await
}
