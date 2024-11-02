use core::str;
use std::{convert::Infallible, env, time::Duration};

use axum::{
    body::{Body, Bytes},
    extract::{MatchedPath, Path, Request as AxumRequest, State},
    http::{HeaderMap, Request, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Json, Router, ServiceExt as AxumServiceExt,
};
use axum_prometheus::PrometheusMetricLayer;
use domain::captcha_like::CaptchaLikeConfig;
use eddist_core::{
    domain::{
        board::{validate_board_key, BoardInfo},
        sjis_str::SJisStr,
    },
    utils::is_prod,
};
use handlebars::Handlebars;
use hyper::{server::conn::http1, service::service_fn};
use hyper_util::rt::{TokioIo, TokioTimer};
use metrics::describe_counter;
use repositories::{bbs_pubsub_repository::RedisPubRepository, bbs_repository::BbsRepositoryImpl};
use routes::{
    auth_code::{get_auth_code, post_auth_code},
    bbs_cgi::post_bbs_cgi,
    dat_routing::{get_dat_txt, get_kako_dat_txt},
};
use services::{
    board_info_service::{BoardInfoServiceInput, BoardInfoServiceOutput},
    thread_list_service::BoardKey,
    AppService, AppServiceContainer,
};
use shiftjis::{SJisResponseBuilder, SjisContentType};
use sqlx::mysql::MySqlPoolOptions;
use tokio::net::TcpListener;
use tower::{util::ServiceExt as ServiceExtTower, Layer};
use tower_http::{
    classify::ServerErrorsFailureClass,
    normalize_path::NormalizePathLayer,
    services::{ServeDir, ServeFile},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::{info_span, Span};
use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};

mod shiftjis;
mod repositories {
    pub(crate) mod bbs_pubsub_repository;
    pub(crate) mod bbs_repository;
}
mod domain {
    pub(crate) mod service {
        pub mod bbscgi_auth_service;
        pub mod board_info_service;
        pub mod ng_word_reading_service;
        pub mod res_creation_span_management_service;
    }

    pub(crate) mod authed_token;
    pub(crate) mod cap;
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
mod services;
pub(crate) mod external {
    pub mod captcha_like_client;
}
mod utils;
mod routes {
    pub mod auth_code;
    pub mod bbs_cgi;
    pub mod dat_routing;
    pub mod statics;
}

#[derive(Clone)]
struct AppState {
    services: AppServiceContainer<BbsRepositoryImpl, RedisPubRepository>,
    tinker_secret: String,
    captcha_like_configs: Vec<CaptchaLikeConfig>,
    template_engine: Handlebars<'static>,
}

impl AppState {
    pub fn get_container(&self) -> &AppServiceContainer<BbsRepositoryImpl, RedisPubRepository> {
        &self.services
    }

    pub fn tinker_secret(&self) -> &str {
        &self.tinker_secret
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if !is_prod() {
        dotenvy::dotenv()?;
    }

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_span_events(FmtSpan::CLOSE)
        .with_ansi(false)
        .init();

    let client = redis::Client::open(env::var("REDIS_URL").unwrap())?;
    let conn_mgr = client.get_connection_manager().await?;
    let pub_repo = RedisPubRepository::new(conn_mgr.clone());

    let pool = MySqlPoolOptions::new()
        .after_connect(|conn, _| {
            use sqlx::Executor;

            // Set transaction isolation level to `READ-COMMITTED`
            Box::pin(async move {
                conn.execute("SET SESSION TRANSACTION ISOLATION LEVEL READ COMMITTED")
                    .await
                    .unwrap();
                log::info!("Set transaction isolation level to `READ-COMMITTED`");
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

    let mut template_engine = Handlebars::new();
    template_engine
        .register_template_string("auth-code.get", include_str!("templates/auth-code.get.hbs"))?;
    template_engine.register_template_string(
        "term-of-usage.get",
        include_str!("templates/term-of-usage.get.hbs"),
    )?;
    template_engine.register_template_string(
        "auth-code.post.success",
        include_str!("templates/auth-code.post.success.hbs"),
    )?;
    template_engine.register_template_string(
        "auth-code.post.failed",
        include_str!("templates/auth-code.post.failed.hbs"),
    )?;

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

    let app_state = AppState {
        services: AppServiceContainer::new(
            BbsRepositoryImpl::new(pool),
            conn_mgr,
            pub_repo,
            *s3_client,
        ),
        tinker_secret,
        captcha_like_configs,
        template_engine,
    };

    let serve_dir = if is_prod() {
        "dist"
    } else {
        "eddist-server/client/dist"
    };
    let serve_file = ServeFile::new(format!("{serve_dir}/index.html"));
    let serve_dir = ServeDir::new(serve_dir).not_found_service(serve_file.clone());

    log::info!("Start application server with 0.0.0.0:8080");
    let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();
    describe_counter!(
        "issue_authed_token",
        "issue authed token count by state and reason if failed"
    );
    describe_counter!("response_creation", "response creation count if success");
    describe_counter!("thread_creation", "thread creation count if success");

    let serve_dir_inner = serve_dir.clone();
    let serve_file_inner = serve_file.clone();
    let serve_file_inner2 = serve_file.clone();

    let app = Router::new()
        .route("/health-check", get(health_check))
        .route("/auth-code", get(get_auth_code).post(post_auth_code))
        .route("/test/bbs.cgi", post(post_bbs_cgi))
        .route("/:boardKey/subject.txt", get(get_subject_txt))
        .route("/:boardKey/head.txt", get(get_head_txt))
        .route("/:boardKey/SETTING.TXT", get(get_setting_txt))
        .route("/:boardKey/dat/:threadId", get(get_dat_txt))
        .route("/:boardKey/kako/:th4/:th5/:threadId", get(get_kako_dat_txt))
        .route("/terms", get(get_term_of_usage))
        .route("/api/boards", get(get_api_boards))
        .route("/metrics", get(|| async move { metric_handle.render() }))
        .route_service(
            "/:boardKey",
            get(|Path(_): Path<String>| async move {
                serve_file_inner
                    .clone()
                    .oneshot(Request::new(Body::empty()))
                    .await
            }),
        )
        .route_service(
            "/:boardKey/:threadId",
            get(
                |Path((board_key, thread_id)): Path<(String, String)>| async move {
                    if board_key == "assets" {
                        // Serve assets
                        serve_dir_inner
                            .clone()
                            .oneshot(
                                Request::builder()
                                    .uri(format!("/assets/{}", thread_id))
                                    .body(Body::empty())
                                    .unwrap(),
                            )
                            .await
                    } else {
                        // Return index.html
                        serve_file_inner2
                            .clone()
                            .oneshot(Request::new(Body::empty()))
                            .await
                    }
                },
            ),
        )
        .route(
            "/test/read.cgi/:boardKey/:threadId",
            get(
                |Path((board_key, thread_id)): Path<(String, String)>| async move {
                    Redirect::permanent(&format!("/{}/{}", board_key, thread_id))
                },
            ),
        )
        .route(
            "/test/read.cgi/:boardKey/:threadId/*pos",
            get(
                |Path((board_key, thread_id)): Path<(String, String)>| async move {
                    Redirect::permanent(&format!("/{}/{}", board_key, thread_id))
                },
            ),
        )
        .nest_service("/dist", serve_dir.clone())
        .fallback_service(serve_dir)
        .with_state(app_state)
        .layer(TimeoutLayer::new(Duration::from_secs(10)))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    // Log the matched route's path (with placeholders not filled in).
                    // Use request.uri() or OriginalUri if you want the real path.
                    let matched_path = request
                        .extensions()
                        .get::<MatchedPath>()
                        .map(MatchedPath::as_str);

                    info_span!(
                        "http_request",
                        method = ?request.method(),
                        matched_path,
                        some_other_field = tracing::field::Empty,
                    )
                })
                .on_request(|_request: &Request<_>, _span: &Span| {
                    // You can use `_span.record("some_other_field", value)` in one of these
                    // closures to attach a value to the initially empty field in the info_span
                    // created above.
                })
                .on_response(|_response: &Response, _latency: Duration, _span: &Span| {
                    // ...
                })
                .on_body_chunk(|_chunk: &Bytes, _latency: Duration, _span: &Span| {
                    // ...
                })
                .on_eos(
                    |_trailers: Option<&HeaderMap>, _stream_duration: Duration, _span: &Span| {
                        // ...
                    },
                )
                .on_failure(
                    |_error: ServerErrorsFailureClass, _latency: Duration, _span: &Span| {
                        // ...
                    },
                ),
        )
        .layer(prometheus_layer);
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

async fn health_check() -> StatusCode {
    StatusCode::OK
}

async fn get_subject_txt(
    State(state): State<AppState>,
    Path(board_key): Path<String>,
) -> impl IntoResponse {
    if validate_board_key(&board_key).is_err() {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    }

    let svc = state.get_container().thread_list();
    let threads = match svc.execute(BoardKey(board_key)).await {
        Ok(threads) => threads,
        Err(e) => {
            return if e.to_string().contains("failed to find board info") {
                Response::builder().status(404).body(Body::empty()).unwrap()
            } else {
                log::error!("Failed to get thread list: {e:?}");
                Response::builder().status(500).body(Body::empty()).unwrap()
            }
        }
    };

    SJisResponseBuilder::new(SJisStr::from_unchecked_vec(threads.get_sjis_thread_list()))
        .content_type(SjisContentType::TextPlain)
        .client_ttl(5)
        .server_ttl(1)
        .build()
        .into_response()
}

async fn get_setting_txt(
    Path(board_key): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if validate_board_key(&board_key).is_err() {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    }

    let BoardInfoServiceOutput {
        board_key,
        name,
        default_name,
        board_info:
            BoardInfo {
                max_thread_name_byte_length,
                max_author_name_byte_length,
                max_email_byte_length,
                max_response_body_byte_length,
                max_response_body_lines,
                ..
            },
    } = state
        .services
        .board_info()
        .execute(BoardInfoServiceInput { board_key })
        .await
        .unwrap();
    let max_response_body_lines = max_response_body_lines / 2;
    let setting_txt = format!(
        "{board_key}@{board_key}
BBS_TITLE={name}
BBS_TITLE_ORIG={name}
BBS_LINE_NUMBER={max_response_body_lines}
BBS_NONAME_NAME={default_name}
BBS_SUBJECT_COUNT={max_thread_name_byte_length}
BBS_NAME_COUNT={max_author_name_byte_length}
BBS_MAIL_COUNT={max_email_byte_length}
BBS_MESSAGE_COUNT={max_response_body_byte_length}"
    );

    SJisResponseBuilder::new((&setting_txt as &str).into())
        .client_ttl(120)
        .server_ttl(300)
        .content_type(SjisContentType::TextPlain)
        .build()
        .into_response()
}

async fn get_head_txt(
    Path(board_key): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if validate_board_key(&board_key).is_err() {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    }

    let BoardInfoServiceOutput {
        board_info: BoardInfo { local_rules, .. },
        ..
    } = state
        .services
        .board_info()
        .execute(BoardInfoServiceInput { board_key })
        .await
        .unwrap();

    SJisResponseBuilder::new((&local_rules as &str).into())
        .client_ttl(120)
        .server_ttl(300)
        .content_type(SjisContentType::TextPlain)
        .build()
        .into_response()
}

async fn get_term_of_usage(State(state): State<AppState>) -> impl IntoResponse {
    let html = state
        .template_engine
        .render(
            "term-of-usage.get",
            &serde_json::json!({
                "domain": env::var("DOMAIN").unwrap_or("example.com".to_string()), 
                "contact_point": env::var("CONTACT_POINT").unwrap_or("abuse@example.com".to_string())
            }),
        )
        .unwrap();

    Html(html)
}

async fn get_api_boards(State(state): State<AppState>) -> impl IntoResponse {
    let svc = state.get_container().list_boards();
    let boards = svc.execute(()).await.unwrap();

    Json(boards)
}
