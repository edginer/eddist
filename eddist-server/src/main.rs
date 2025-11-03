use std::{convert::Infallible, env, time::Duration};

use axum::{
    body::Body, extract::Request as AxumRequest, response::Response, ServiceExt as AxumServiceExt,
};
use domain::captcha_like::CaptchaLikeConfig;
use eddist_core::{tracing::init_tracing, utils::is_prod};
use hyper::{server::conn::http1, service::service_fn};
use hyper_util::rt::{TokioIo, TokioTimer};
use metrics::describe_counter;
use repositories::{
    bbs_pubsub_repository::RedisPubRepository, bbs_repository::BbsRepositoryImpl,
    idp_repository::IdpRepositoryImpl, user_repository::UserRepositoryImpl,
    user_restriction_repository::UserRestrictionRepositoryImpl,
};
use services::{user_restriction_service::start_cache_refresh_task, AppServiceContainer};
use sqlx::mysql::MySqlPoolOptions;
use template::load_template_engine;
use tokio::net::TcpListener;
use tower::Layer;
use tower_http::{
    normalize_path::NormalizePathLayer,
    services::{ServeDir, ServeFile},
};

use crate::app::{create_app, AppState};

pub mod app;
mod shiftjis;
mod repositories {
    pub(crate) mod bbs_pubsub_repository;
    pub(crate) mod bbs_repository;
    pub(crate) mod idp_repository;
    pub(crate) mod user_repository;
    pub(crate) mod user_restriction_repository;
}
mod domain {
    pub(crate) mod service {
        pub mod bbscgi_auth_service;
        pub mod bbscgi_user_reg_temp_url_service;
        pub mod board_info_service;
        pub mod email_auth_restriction_service;
        pub mod ng_word_reading_service;
        pub mod oidc_client_service;
        pub mod res_creation_span_management_service;
    }

    pub(crate) mod user;

    pub(crate) mod authed_token;
    pub(crate) mod captcha_like;
    pub(crate) mod metadent;
    pub(crate) mod ng_word;
    pub(crate) mod res;
    pub(crate) mod res_core;
    pub(crate) mod thread;
    pub(crate) mod thread_list;
    pub(crate) mod thread_res_list;

    pub(crate) mod utils;
}
mod error;
mod middleware;
mod services;
mod template;

pub(crate) mod external {
    pub mod captcha_like_client;
    pub mod oidc_client;
}
pub(crate) mod utils;
mod routes {
    pub mod auth_code;
    pub mod bbs_cgi;
    pub mod dat_routing;
    pub mod statics;
    pub mod subject_list;
    pub mod user;
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if !is_prod() {
        dotenvy::dotenv()?;
    }

    init_tracing();

    let client = redis::Client::open(env::var("REDIS_URL").unwrap())?;
    let conn_mgr = client.get_connection_manager().await?;
    let pub_repo = RedisPubRepository::new(conn_mgr.clone());

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

    let captcha_like_configs_path =
        env::var("CAPTCHA_CONFIG_PATH").unwrap_or("./captcha-config.json".to_string());
    let captcha_like_configs = std::fs::read_to_string(captcha_like_configs_path)?;
    let captcha_like_configs =
        serde_json::from_str::<Vec<CaptchaLikeConfig>>(&captcha_like_configs)?;

    let serve_dir = if is_prod() {
        "dist"
    } else {
        "eddist-server/client/dist"
    };

    let template_engine = load_template_engine(&format!("{serve_dir}/index.html"));

    let serve_file = ServeFile::new(format!("{serve_dir}/index.html"));
    let serve_dir = ServeDir::new(serve_dir).not_found_service(serve_file.clone());

    let s3_client = s3::bucket::Bucket::new(
        env::var("S3_BUCKET_NAME").unwrap().trim(),
        s3::Region::R2 {
            account_id: env::var("R2_ACCOUNT_ID").unwrap().trim().to_string(),
        },
        s3::creds::Credentials::new(
            Some(env::var("S3_ACCESS_KEY").unwrap().trim()),
            Some(env::var("S3_ACCESS_SECRET_KEY").unwrap().trim()),
            None,
            None,
            None,
        )
        .unwrap(),
    )
    .unwrap();

    let user_restriction_repo = UserRestrictionRepositoryImpl::new(pool.clone());

    let app_state = AppState {
        services: AppServiceContainer::new(
            BbsRepositoryImpl::new(pool.clone()),
            UserRepositoryImpl::new(pool.clone()),
            IdpRepositoryImpl::new(pool.clone()),
            user_restriction_repo.clone(),
            conn_mgr.clone(),
            pub_repo,
            *s3_client,
        ),
        tinker_secret,
        captcha_like_configs,
        template_engine,
    };

    // Start background task for user restriction cache refresh
    start_cache_refresh_task(user_restriction_repo, Duration::from_secs(300));

    log::info!("Start application server with 0.0.0.0:8080");

    describe_counter!(
        "issue_authed_token",
        "issue authed token count by state and reason if failed"
    );
    describe_counter!("response_creation", "response creation count if success");
    describe_counter!("thread_creation", "thread creation count if success");

    let serve_dir_inner = serve_dir.clone();

    let app = create_app(app_state, conn_mgr, serve_dir, serve_dir_inner);

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
