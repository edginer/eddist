use core::str;
use std::{collections::HashMap, convert::Infallible, env, time::Duration};

use axum::{
    body::{Body, Bytes},
    extract::{MatchedPath, Path, Request as AxumRequest, State},
    http::{HeaderMap, Request, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Form, Json, Router, ServiceExt as AxumServiceExt,
};
use axum_extra::extract::CookieJar;
use axum_prometheus::PrometheusMetricLayer;
use base64::Engine;
use chrono::Utc;
use domain::captcha_like::CaptchaLikeConfig;
use eddist_core::{
    domain::{board::BoardInfo, sjis_str::SJisStr, tinker::Tinker},
    utils::is_prod,
};
use error::{BbsCgiError, BbsPostAuthWithCodeError, InsufficientParamType, InvalidParamType};
use handlebars::Handlebars;
use hyper::{server::conn::http1, service::service_fn};
use hyper_util::rt::{TokioIo, TokioTimer};
use jsonwebtoken::EncodingKey;
use jwt_simple::prelude::MACLike;
use metrics::describe_counter;
use repositories::{bbs_pubsub_repository::RedisPubRepository, bbs_repository::BbsRepositoryImpl};
use serde_json::json;
use services::{
    auth_with_code_service::{AuthWithCodeServiceInput, AuthWithCodeServiceOutput},
    board_info_service::{BoardInfoServiceInput, BoardInfoServiceOutput},
    kako_thread_retrieval_service::KakoThreadRetrievalServiceInput,
    res_creation_service::{ResCreationServiceInput, ResCreationServiceOutput},
    thread_creation_service::{TheradCreationServiceInput, ThreadCreationServiceOutput},
    thread_list_service::BoardKey,
    thread_retrieval_service::ThreadRetrievalServiceInput,
    AppService, AppServiceContainer, BbsCgiService,
};
use shiftjis::{shift_jis_url_encodeded_body_to_vec, SJisResponseBuilder, SjisContentType};
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
use tracing::{error_span, info_span, warn_span, Span};
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
                        serve_file_inner
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

async fn get_dat_txt(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((board_key, thread_id_with_dat)): Path<(String, String)>,
) -> Response {
    if thread_id_with_dat.len() != 14 {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    }
    let thread_number = thread_id_with_dat.replace(".dat", "");
    let Ok(thread_number_num) = thread_number.parse::<i64>() else {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    };

    let svc = state.get_container().thread_retrival();
    let result = match svc
        .execute(ThreadRetrievalServiceInput {
            board_key: board_key.clone(),
            thread_number: thread_number_num as u64,
        })
        .await
    {
        Ok(raw) => raw,
        Err(e) => {
            if e.root_cause()
                .to_string()
                .contains("cannot find such thread")
            {
                let current_unix_epoch = Utc::now().timestamp();
                return if thread_number_num > current_unix_epoch {
                    // Not found response
                    Response::builder().status(404).body(Body::empty()).unwrap()
                } else {
                    // Redirect to kako thread
                    Response::builder()
                        .status(302)
                        .header(
                            "Location",
                            format!(
                                "/{}/kako/{}/{}/{}.dat",
                                board_key,
                                &thread_number[0..4],
                                &thread_number[0..5],
                                thread_number
                            ),
                        )
                        .body(Body::empty())
                        .unwrap()
                };
            } else {
                // Not found response
                return Response::builder().status(404).body(Body::empty()).unwrap();
            }
        }
    };

    let range = headers.get("Range");
    let ua = headers.get("User-Agent").map(|x| x.to_str().unwrap());

    let (result, is_partial) = match (range, ua) {
        (Some(range), Some(ua)) if !ua.contains("Xeno") => {
            let range = range.to_str().unwrap();
            if let Some(range) = range.split('=').nth(1) {
                let range = range.split('-').collect::<Vec<_>>();
                let Some(start) = range.first().and_then(|x| x.parse::<usize>().ok()) else {
                    return Response::builder().status(400).body(Body::empty()).unwrap();
                };

                let raw = result.raw().into_iter().skip(start).collect::<Vec<_>>();
                (raw, true)
            } else {
                (result.raw(), false)
            }
        }
        _ => (result.raw(), false),
    };

    SJisResponseBuilder::new(SJisStr::from_unchecked_vec(result))
        .content_type(SjisContentType::TextPlain)
        .client_ttl(5)
        .server_ttl(1)
        .status_code(if is_partial {
            StatusCode::PARTIAL_CONTENT
        } else {
            StatusCode::OK
        })
        .build()
        .into_response()
}

async fn get_kako_dat_txt(
    State(state): State<AppState>,
    Path((board_key, _, _, thread_id_with_dat)): Path<(String, String, String, String)>,
) -> Response {
    if thread_id_with_dat.len() != 14 {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    }
    let thread_number = thread_id_with_dat.replace(".dat", "");

    let svc = state.get_container().kako_thread_retrieval();

    let result = match svc
        .execute(KakoThreadRetrievalServiceInput {
            board_key,
            thread_number,
        })
        .await
    {
        Ok(result) => result,
        Err(err) => {
            return if err.to_string().contains("Thread not found") {
                Response::builder().status(404).body(Body::empty()).unwrap()
            } else {
                Response::builder().status(500).body(Body::empty()).unwrap()
            };
        }
    };

    let sjis_str = if let Ok(result) = str::from_utf8(&result) {
        SJisStr::from(result)
    } else {
        SJisStr::from_unchecked_vec(result)
    };

    SJisResponseBuilder::new(sjis_str)
        .content_type(SjisContentType::TextPlain)
        .server_ttl(3600)
        .build()
        .into_response()
}

async fn get_setting_txt(
    Path(board_key): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
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

// NOTE: this system will be changed in the future
async fn get_auth_code(State(state): State<AppState>) -> impl IntoResponse {
    let site_keys = state
        .captcha_like_configs
        .iter()
        .filter_map(|config| match config {
            CaptchaLikeConfig::Turnstile { site_key, .. } => Some(("cf_site_key", site_key)),
            CaptchaLikeConfig::Hcaptcha { site_key, .. } => Some(("hcaptcha_site_key", site_key)),
            CaptchaLikeConfig::Monocle { site_key, .. } => Some(("monocle_site_key", site_key)),
            _ => {
                warn_span!(
                    "not implemented yet such captcha like config, ignored",
                    ?config
                );
                None
            }
        })
        .collect::<HashMap<_, _>>();

    let html = state
        .template_engine
        .render("auth-code.get", &serde_json::json!(site_keys))
        .unwrap();

    let mut resp = Html(html).into_response();
    let headers = resp.headers_mut();
    headers.insert("Cache-Control", "private".parse().unwrap());

    resp
}

async fn post_auth_code(
    headers: HeaderMap,
    State(state): State<AppState>,
    Form(form): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    let token = match state
        .services
        .auth_with_code()
        .execute(AuthWithCodeServiceInput {
            code: form["auth-code"].to_string(),
            origin_ip: get_origin_ip(&headers).to_string(),
            user_agent: get_ua(&headers).to_string(),
            captcha_like_configs: state.captcha_like_configs.clone(),
            responses: form,
        })
        .await
    {
        Ok(AuthWithCodeServiceOutput { token }) => token,
        Err(e) => {
            return if let Some(e) = e.downcast_ref::<BbsPostAuthWithCodeError>() {
                Html(
                    state
                        .template_engine
                        .render("auth-code.post.failed", &json!({ "reason": e.to_string() }))
                        .unwrap(),
                )
            } else {
                log::error!("Failed to issue authed token: {e:?}");

                Html(
                    state
                        .template_engine
                        .render(
                            "auth-code.post.failed",
                            &json!({ "reason": "不明な理由です（認証に失敗した可能性があります）" }),
                        )
                        .unwrap(),
                )
            }
        }
    };

    let html = state
        .template_engine
        .render("auth-code.post.success", &json!({ "token": token }))
        .unwrap();

    Html(html)
}

async fn post_bbs_cgi(
    headers: HeaderMap,
    jar: CookieJar,
    State(state): State<AppState>,
    body: String,
) -> Response {
    let form = shift_jis_url_encodeded_body_to_vec(&body).unwrap();
    let is_thread = {
        let Some(submit) = form.get("submit") else {
            return BbsCgiError::from(InsufficientParamType::Submit).into_response();
        };

        match submit as &str {
            "書き込む" => false,
            "新規スレッド作成" => true,
            _ => return BbsCgiError::from(InvalidParamType::Submit).into_response(),
        }
    };

    let origin_ip = get_origin_ip(&headers);
    let ua = get_ua(&headers);
    let asn_num = get_asn_num(&headers);
    let tinker = jar
        .get("tinker-token")
        .and_then(|x| get_tinker(x.value(), state.tinker_secret()));
    let edge_token = jar.get("edge-token").map(|x| x.value().to_string());

    let Some(board_key) = form.get("bbs").map(|x| x.to_string()) else {
        return BbsCgiError::from(InsufficientParamType::Bbs).into_response();
    };
    let Some(name) = form.get("FROM").map(|x| x.to_string()) else {
        return BbsCgiError::from(InsufficientParamType::From).into_response();
    };
    let Some(mail) = form.get("mail").map(|x| x.to_string()) else {
        return BbsCgiError::from(InsufficientParamType::Mail).into_response();
    };
    let Some(body) = form.get("MESSAGE").map(|x| x.to_string()) else {
        return BbsCgiError::from(InsufficientParamType::Body).into_response();
    };

    fn on_error(e: BbsCgiError) -> Response {
        if matches!(e, BbsCgiError::Other(_)) {
            error_span!("thread_creation_error", error = ?e);
        }
        let is_cookie_reset = matches!(e, BbsCgiError::InvalidAuthedToken);
        let mut resp = e.into_response();
        if is_cookie_reset {
            resp.headers_mut().append(
                "Set-Cookie",
                "edge-token = ; Max-Age = 0; Path = /; "
                    .to_string()
                    .parse()
                    .unwrap(),
            );
            resp.headers_mut().append(
                "Set-Cookie",
                "tinker = ; Max-Age = 0; Path = /; "
                    .to_string()
                    .parse()
                    .unwrap(),
            );
        }
        resp
    }

    let tinker = if is_thread {
        let Some(title) = form.get("subject").map(|x| x.to_string()) else {
            return BbsCgiError::from(InsufficientParamType::Subject).into_response();
        };

        let svc = state.services.thread_creation();
        match svc
            .execute(TheradCreationServiceInput {
                board_key,
                title,
                authed_token: edge_token,
                name,
                mail,
                body,
                tinker,
                ip_addr: origin_ip.to_string(),
                user_agent: ua.to_string(),
                asn_num,
            })
            .await
        {
            Ok(ThreadCreationServiceOutput { tinker }) => tinker,
            Err(e) => {
                return on_error(e);
            }
        }
    } else {
        let Some(thread_number) = form.get("key").map(|x| x.to_string()) else {
            return BbsCgiError::from(InsufficientParamType::Key).into_response();
        };
        let Ok(thread_number) = thread_number.parse::<u64>() else {
            return BbsCgiError::from(InvalidParamType::Key).into_response();
        };

        let svc = state.services.res_creation();
        match svc
            .execute(ResCreationServiceInput {
                board_key,
                thread_number,
                authed_token_cookie: edge_token,
                name,
                mail,
                body,
                tinker,
                ip_addr: origin_ip.to_string(),
                user_agent: ua.to_string(),
                asn_num,
            })
            .await
        {
            Ok(ResCreationServiceOutput { tinker }) => tinker,
            Err(e) => {
                return on_error(e);
            }
        }
    };

    SJisResponseBuilder::new(SJisStr::from(
        r#"<html><!-- 2ch_X:true -->
<head>
    <meta http-equiv="Content-Type" content="text/html; charset=x-sjis">
    <title>書きこみました</title>
</head>
<body>書きこみました</body>
</html>"#,
    ))
    .content_type(SjisContentType::TextHtml)
    .add_set_cookie(
        "tinker-token".to_string(),
        jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &tinker,
            &EncodingKey::from_base64_secret(state.tinker_secret()).unwrap(),
        )
        .unwrap(),
        time::Duration::days(365),
    )
    .add_set_cookie(
        "edge-token".to_string(),
        tinker.authed_token().to_string(),
        time::Duration::days(365),
    )
    .build()
    .into_response()
}

async fn get_api_boards(State(state): State<AppState>) -> impl IntoResponse {
    let svc = state.get_container().list_boards();
    let boards = svc.execute(()).await.unwrap();

    Json(boards)
}

fn get_origin_ip(headers: &HeaderMap) -> &str {
    let origin_ip = headers
        .get("Cf-Connecting-IP")
        .or_else(|| headers.get("X-Forwarded-For"))
        .map(|x| x.to_str());

    if is_prod() {
        origin_ip.unwrap().unwrap()
    } else {
        origin_ip.unwrap_or(Ok("localhost")).unwrap()
    }
}

fn get_ua(headers: &HeaderMap) -> &str {
    headers
        .get("User-Agent")
        .map(|x| x.to_str())
        .unwrap_or(Ok("unknown"))
        .unwrap()
}

fn get_asn_num(headers: &HeaderMap) -> u32 {
    let header_name = env::var("ASN_NUMBER_HEADER_NAME").unwrap_or("X-ASN-Num".to_string());

    let header = headers.get(header_name).map(|x| x.to_str());

    if is_prod() {
        header.unwrap().unwrap().parse::<u32>().unwrap()
    } else {
        header.unwrap_or(Ok("0")).unwrap().parse::<u32>().unwrap()
    }
}

fn get_tinker(tinker: &str, secret: &str) -> Option<Tinker> {
    let key = jwt_simple::prelude::HS256Key::from_bytes(
        &base64::engine::general_purpose::STANDARD
            .decode(secret.trim())
            .unwrap(),
    );
    let tinker = key.verify_token::<Tinker>(tinker, None).ok()?;

    Some(tinker.custom)
}
