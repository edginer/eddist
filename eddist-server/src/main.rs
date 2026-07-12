#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::{convert::Infallible, env, sync::Arc, time::Duration};

use axum::{
    ServiceExt as AxumServiceExt, body::Body, extract::Request as AxumRequest, response::Response,
};
use eddist::{
    AppState,
    app::create_app,
    load_template_engine,
    middleware::not_found_rate_limit::NotFoundPenaltyCache,
    repositories::{
        bbs_pubsub_repository::{RedisCreationEventRepository, RedisPubRepository},
        bbs_repository::BbsRepositoryImpl,
        captcha_config_repository::CaptchaConfigRepositoryImpl,
        idp_repository::IdpRepositoryImpl,
        notice_repository::NoticeRepositoryImpl,
        stats_repository::StatsRepositoryImpl,
        terms_repository::TermsRepositoryImpl,
        user_repository::UserRepositoryImpl,
        user_restriction_repository::UserRestrictionRepositoryImpl,
    },
    services::{
        AppServiceContainer, PubSubRepos,
        captcha_config_cache::{refresh_captcha_config_cache, start_captcha_config_refresh_task},
        server_settings_cache::{
            refresh_server_settings_cache, start_server_settings_refresh_task,
        },
        stats_counter::{flush_stats_now, start_stats_flush_task},
    },
    start_cache_refresh_task,
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

    let client = redis::Client::open(env::var("REDIS_URL").unwrap())?;
    let conn_mgr = client.get_connection_manager().await?;
    let pub_repo = RedisPubRepository::new(conn_mgr.clone());
    let event_repo = RedisCreationEventRepository::new(conn_mgr.clone());

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
        .connect(&env::var("DATABASE_URL")?)
        .await?;

    let tinker_secret = env::var("TINKER_SECRET").unwrap();

    // Load initial captcha configs from database and initialize cache
    refresh_captcha_config_cache(&CaptchaConfigRepositoryImpl::new(pool.clone())).await?;

    let template_engine = load_template_engine();

    let r2_account_id = env::var("R2_ACCOUNT_ID").unwrap();
    let s3_bucket_name = env::var("S3_BUCKET_NAME").unwrap().trim().to_string();
    let s3_endpoint = format!("https://{}.r2.cloudflarestorage.com", r2_account_id.trim());
    let s3_creds = aws_sdk_s3::config::Credentials::new(
        env::var("S3_ACCESS_KEY").unwrap().trim(),
        env::var("S3_ACCESS_SECRET_KEY").unwrap().trim(),
        None,
        None,
        "custom",
    );
    let s3_config = aws_sdk_s3::Config::builder()
        .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
        .credentials_provider(s3_creds)
        .region(aws_sdk_s3::config::Region::new("auto"))
        .endpoint_url(s3_endpoint)
        .build();
    let s3_client = aws_sdk_s3::Client::from_conf(s3_config);

    let user_restriction_repo = UserRestrictionRepositoryImpl::new(pool.clone());
    let notice_repo = NoticeRepositoryImpl::new(pool.clone());
    let terms_repo = TermsRepositoryImpl::new(pool.clone());
    let stats_repo = StatsRepositoryImpl::new(pool.clone());
    let stats_repo_for_flush = stats_repo.clone();
    let stats_repo_for_shutdown = stats_repo.clone();

    // Load initial server settings from database and initialize cache
    refresh_server_settings_cache(&pool).await?;

    let app_state = AppState {
        services: AppServiceContainer::new(
            BbsRepositoryImpl::new(pool.clone()),
            UserRepositoryImpl::new(pool.clone()),
            IdpRepositoryImpl::new(pool.clone()),
            user_restriction_repo.clone(),
            conn_mgr.clone(),
            PubSubRepos {
                pub_repo,
                event_repo,
            },
            s3_client,
            s3_bucket_name,
        ),
        notice_repo,
        terms_repo,
        stats_repo,
        template_engine: Arc::new(template_engine),
        tinker_secret,
        redis_conn: conn_mgr.clone(),
        not_found_penalty_cache: NotFoundPenaltyCache::new(),
    };

    // Start background task for user restriction cache refresh
    start_cache_refresh_task(user_restriction_repo, Duration::from_secs(300));

    // Start background task for captcha config cache refresh (every 5 minutes)
    start_captcha_config_refresh_task(pool.clone(), Duration::from_secs(300));

    // Start background task for server settings cache refresh (every 5 minutes)
    start_server_settings_refresh_task(pool.clone(), Duration::from_secs(300));

    // Start background task for stats flush (every 30 seconds)
    start_stats_flush_task(stats_repo_for_flush, Duration::from_secs(30));

    log::info!("Start application server with 0.0.0.0:8080");

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

    let app = create_app(app_state, conn_mgr);

    let listener = TcpListener::bind((
        "0.0.0.0",
        env::var("PORT")
            .as_deref()
            .unwrap_or("8080")
            .parse::<u16>()
            .unwrap(),
    ))
    .await
    .unwrap();

    let app = NormalizePathLayer::trim_trailing_slash().layer(app);
    let app = AxumServiceExt::<AxumRequest>::into_make_service(app);
    axum::serve(listener, app)
        .with_graceful_shutdown(graceful_shutdown_http())
        .await
        .unwrap();

    if flush_stats_now(&stats_repo_for_shutdown).await.is_ok() {
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
