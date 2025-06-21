use core::str;
use std::{convert::Infallible, env, time::Duration};

use axum::{
    body::{Body, Bytes},
    extract::{MatchedPath, Path, Request as AxumRequest, State},
    http::{HeaderMap, Request, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Extension, Json, Router, ServiceExt as AxumServiceExt,
};
use axum_prometheus::PrometheusMetricLayer;
use domain::captcha_like::CaptchaLikeConfig;
use eddist_core::{
    domain::board::{validate_board_key, BoardInfo},
    tracing::init_tracing,
    utils::is_prod,
};
use handlebars::Handlebars;
use hyper::{server::conn::http1, service::service_fn};
use hyper_util::rt::{TokioIo, TokioTimer};
use metrics::describe_counter;
use repositories::{
    bbs_pubsub_repository::RedisPubRepository, bbs_repository::BbsRepositoryImpl,
    idp_repository::IdpRepositoryImpl, user_repository::UserRepositoryImpl,
};
use routes::{
    auth_code::{get_auth_code, post_auth_code},
    bbs_cgi::post_bbs_cgi,
    dat_routing::{get_dat_txt, get_kako_dat_txt},
    subject_list::{get_subject_txt, get_subject_txt_with_metadent},
    user::user_routes,
};
use services::{
    board_info_service::{BoardInfoServiceInput, BoardInfoServiceOutput},
    AppService, AppServiceContainer,
};
use shiftjis::{SJisResponseBuilder, SjisContentType};
use sqlx::mysql::MySqlPoolOptions;
use template::load_template_engine;
use tokio::net::TcpListener;
use tower::{util::ServiceExt as ServiceExtTower, Layer};
use tower_http::{
    catch_panic::CatchPanicLayer,
    classify::ServerErrorsFailureClass,
    normalize_path::NormalizePathLayer,
    services::{ServeDir, ServeFile},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::{info_span, Span};
use utils::CsrfState;

mod shiftjis;
mod repositories {
    pub(crate) mod bbs_pubsub_repository;
    pub(crate) mod bbs_repository;
    pub(crate) mod idp_repository;
    pub(crate) mod user_repository;
}
mod domain {
    pub(crate) mod service {
        pub mod bbscgi_auth_service;
        pub mod bbscgi_user_reg_temp_url_service;
        pub mod board_info_service;
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

#[derive(Clone)]
struct AppState {
    services: AppServiceContainer<
        BbsRepositoryImpl,
        UserRepositoryImpl,
        IdpRepositoryImpl,
        RedisPubRepository,
    >,
    tinker_secret: String,
    captcha_like_configs: Vec<CaptchaLikeConfig>,
    template_engine: Handlebars<'static>,
}

impl AppState {
    pub fn get_container(
        &self,
    ) -> &AppServiceContainer<
        BbsRepositoryImpl,
        UserRepositoryImpl,
        IdpRepositoryImpl,
        RedisPubRepository,
    > {
        &self.services
    }

    pub fn tinker_secret(&self) -> &str {
        &self.tinker_secret
    }
}

fn render_index_html(
    template_engine: &Handlebars<'static>,
    canonical: Option<String>,
) -> impl IntoResponse {
    let mut resp = Html(
        template_engine
            .render(
                "dist-index_html",
                &serde_json::json!({
                    "bbs_name": env::var("BBS_NAME").unwrap_or("エッヂ掲示板".to_string()),
                    "canonical": canonical,
                    "available_user_registration": env::var("ENABLE_USER_REGISTRATION")
                        .ok()
                        .map(|v| v == "true")
                        .unwrap_or(false)
                        .to_string(),
                }),
            )
            .unwrap(),
    )
    .into_response();

    resp.headers_mut()
        .insert("Cache-Control", "s-maxage=300".parse().unwrap());
    resp
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
                log::info!("Set TIME_TRUNCATE_FRACTIONAL mode to match chrono truncation behavior");
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

    let app_state = AppState {
        services: AppServiceContainer::new(
            BbsRepositoryImpl::new(pool.clone()),
            UserRepositoryImpl::new(pool.clone()),
            IdpRepositoryImpl::new(pool),
            conn_mgr.clone(),
            pub_repo,
            *s3_client,
        ),
        tinker_secret,
        captcha_like_configs,
        template_engine,
    };

    log::info!("Start application server with 0.0.0.0:8080");
    let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();
    describe_counter!(
        "issue_authed_token",
        "issue authed token count by state and reason if failed"
    );
    describe_counter!("response_creation", "response creation count if success");
    describe_counter!("thread_creation", "thread creation count if success");

    let serve_dir_inner = serve_dir.clone();

    let app = Router::new()
        .route("/health-check", get(health_check))
        .route("/robots.txt", get(get_robots_txt))
        .route("/auth-code", get(get_auth_code).post(post_auth_code))
        .route("/test/bbs.cgi", post(post_bbs_cgi))
        .route("/{boardKey}/subject.txt", get(get_subject_txt))
        .route(
            "/{boardKey}/subject-metadent.txt",
            get(get_subject_txt_with_metadent),
        )
        .route("/{boardKey}/head.txt", get(get_head_txt))
        .route("/{boardKey}/SETTING.TXT", get(get_setting_txt))
        .route("/{boardKey}/dat/{threadId}", get(get_dat_txt))
        .route(
            "/{boardKey}/kako/{th4}/{th5}/{threadId}",
            get(get_kako_dat_txt),
        )
        .route("/terms", get(get_term_of_usage))
        .route("/api/terms", get(get_api_terms))
        .route("/api/boards", get(get_api_boards))
        .nest("/user", user_routes())
        .route("/metrics", get(|| async move { metric_handle.render() }))
        .route(
            "/{boardKey}",
            get(
                |State(state): State<AppState>, Path(board_key): Path<String>| async move {
                    render_index_html(
                        &state.template_engine,
                        env::var("BASE_URL")
                            .ok()
                            .map(|base_url| format!("{base_url}/{board_key}")),
                    )
                },
            ),
        )
        .route(
            "/",
            get(|State(state): State<AppState>| async move {
                render_index_html(&state.template_engine, env::var("BASE_URL").ok())
            }),
        )
        .route_service(
            "/assets/{item}",
            get(|Path(item): Path<String>| async move {
                serve_dir_inner
                    .clone()
                    .oneshot(
                        Request::builder()
                            .uri(format!("/assets/{}", item))
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
            }),
        )
        .route(
            "/{boardKey}/{threadId}",
            get(
                |State(app_state): State<AppState>,
                 Path((board_key, thread_id)): Path<(String, String)>| async move {
                    render_index_html(
                        &app_state.template_engine,
                        env::var("BASE_URL")
                            .ok()
                            .map(|base_url| format!("{base_url}/{board_key}/{thread_id}")),
                    )
                },
            ),
        )
        .route(
            "/test/read.cgi/{boardKey}/{threadId}",
            get(
                |Path((board_key, thread_id)): Path<(String, String)>| async move {
                    Redirect::permanent(&format!("/{}/{}", board_key, thread_id))
                },
            ),
        )
        .route(
            "/test/read.cgi/{boardKey}/{threadId}/{*pos}",
            get(
                |Path((board_key, thread_id, _)): Path<(String, String, String)>| async move {
                    Redirect::permanent(&format!("/{}/{}", board_key, thread_id))
                },
            ),
        )
        .nest_service("/dist", serve_dir.clone())
        .fallback_service(serve_dir)
        .with_state(app_state)
        .layer(CatchPanicLayer::custom(|e| {
            tracing::error!("Panic: {e:?}");
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Internal Server Error"))
                .unwrap()
        }))
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
        .layer(Extension(CsrfState::new(conn_mgr)));

    let app = if env::var("AXUM_METRICS") == Ok("true".to_string()) {
        app.layer(prometheus_layer)
    } else {
        app
    };
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

async fn get_setting_txt(
    Path(board_key): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if validate_board_key(&board_key).is_err() {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    }

    let Ok(BoardInfoServiceOutput {
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
    }) = state
        .services
        .board_info()
        .execute(BoardInfoServiceInput { board_key })
        .await
    else {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    };
    let max_response_body_lines = max_response_body_lines / 2;

    let setting_txt = state
        .template_engine
        .render(
            "setting-txt.get",
            &serde_json::json!({
                "board_key": board_key,
                "name": name,
                "default_name": default_name,
                "max_thread_name_byte_length": max_thread_name_byte_length,
                "max_author_name_byte_length": max_author_name_byte_length,
                "max_email_byte_length": max_email_byte_length,
                "max_response_body_byte_length": max_response_body_byte_length,
                "max_response_body_lines": max_response_body_lines,
            }),
        )
        .unwrap();

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

    let Ok(BoardInfoServiceOutput {
        board_info: BoardInfo { local_rules, .. },
        ..
    }) = state
        .services
        .board_info()
        .execute(BoardInfoServiceInput { board_key })
        .await
    else {
        return Response::builder().status(404).body(Body::empty()).unwrap();
    };

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

    let mut resp = Html(html).into_response();
    resp.headers_mut()
        .insert("Cache-Control", "s-maxage=300".parse().unwrap());
    resp
}

async fn get_api_boards(State(state): State<AppState>) -> impl IntoResponse {
    let svc = state.get_container().list_boards();
    let boards = svc.execute(()).await.unwrap();

    let mut resp = Json(boards).into_response();
    resp.headers_mut()
        .insert("Cache-Control", "s-maxage=300".parse().unwrap());
    resp
}

async fn get_api_terms() -> impl IntoResponse {
    let domain = env::var("DOMAIN").unwrap_or("example.com".to_string());
    let contact_point = env::var("CONTACT_POINT").unwrap_or("abuse@example.com".to_string());
    let terms_content = serde_json::json!({
        "sections": [
            {
                "title": "第1条（適用範囲）",
                "content": "本利用規約（以下「本規約」といいます）は、当掲示板（以下「本サービス」といいます）を利用するすべてのユーザー（以下「利用者」といいます）に適用されます。利用者は、本サービスを利用することにより、本規約に同意したものとみなされます。"
            },
            {
                "title": "第2条（収集する情報）",
                "content": "本サービスは、利用者のIPアドレス、Cookie、その他端末を特定するための情報を収集し、以下の目的で使用します。",
                "list": [
                    "本サービスの運営及び管理",
                    "不正利用の防止及びセキュリティの向上",
                    "サービスの改善及び提供内容の最適化"
                ],
                "additional": "これらの情報は、本サービス運営のためにのみ利用され、以下の場合に加えて法執行機関等からの正当な要求に応じる場合、または利用者が同意した場合を除き、第三者に提供することはありません。",
                "additional_list": [
                    "書き込み時、また書き込み前の認証時に利用者の正当性を確認するために、いくつかのサービス(*1)に問い合わせる場合"
                ]
            },
            {
                "title": "第3条（書き込みの責任）",
                "content": "本サービスにおけるすべての書き込み（テキスト、画像、その他の情報を含む）は、その書き込みを行った利用者に全責任が属します。利用者は、以下に定める違法な書き込みや不適切な内容を投稿しないことに同意するものとします。"
            },
            {
                "title": "第4条（禁止事項）",
                "content": "利用者は、以下の行為を行ってはなりません。",
                "sections": [
                    {
                        "subtitle": "違法な書き込み",
                        "items": [
                            "名誉毀損、中傷、侮辱、脅迫など、他者の権利や名誉を侵害する内容",
                            "著作権、商標権、特許権、プライバシー権、肖像権などの知的財産権を侵害する内容",
                            "無断で個人情報（氏名、住所、電話番号、メールアドレスなど）を公開する行為",
                            "法律で禁止されている行為を助長する内容や、犯罪行為に関与する内容"
                        ]
                    },
                    {
                        "subtitle": "その他不適切な書き込み",
                        "items": [
                            "過度に暴力的な表現、残虐な表現、児童ポルノを含む内容",
                            "虚偽の情報を流布し、混乱や誤解を招く内容",
                            "スパム、商業目的の宣伝、不正アクセス行為に関与する内容",
                            "スクリプトやbotなどを用いた自動書き込み行為",
                            "人種、民族、国籍、性別、宗教、障害、性的指向などに対する差別的な発言"
                        ]
                    }
                ]
            },
            {
                "title": "第5条（著作権）",
                "content": "利用者が本サービスに投稿した書き込みの著作権は、書き込みを行った利用者自身に属します。ただし、利用者は本サービス及び本サービスの関連サービス(*2)で投稿内容を使用、複製、編集、公開することについて、運営者に対して無期限かつ無償で非独占的な使用権を付与し、著作者人格権を行使しないことに同意します。利用者は、利用者自身の書き込みが第三者によって無断で転載されることを防止するため、本サービスに書き込みを行う際には原則、本サービスならびに本サービスに関連するサービス以外への転載を許諾しないものとして書き込むことに同意します。"
            },
            {
                "title": "第6条（違反行為への対応）",
                "content": "本サービスの運営側は、利用者の書き込み内容が本規約に違反している、または不適切であると判断した場合、当該書き込みを事前通知なく削除する権利を有します。また、法執行機関や、名誉毀損や中傷に関する被害者からの正当な求めがあった場合、投稿内容の削除および発信者情報の開示に応じることがあります。また、違反行為を繰り返す利用者に対してはアカウントの一時停止などの措置を取ることがあります。"
            },
            {
                "title": "第7条（免責事項）",
                "content": "本サービスは、利用者が本サービスの利用に関連して被ったあらゆる損害等について、一切の責任を負いません。利用者は、自己の責任で本サービスを利用するものとし、運営側に対して一切の賠償請求を行わないものとします。"
            },
            {
                "title": "第8条（規約の改定）",
                "content": "本規約は、必要に応じて改定されることがあります。改定後の規約は、本サービス上に掲載された時点で効力を発生します。利用者は、定期的に本規約を確認する義務を負い、改定後も本サービスの利用を継続した場合、改定内容に同意したものとみなされます。"
            }
        ],
        "footnotes": [
            "例: hCaptcha, Cloudflare Turnstile, Spur",
            format!("本サービスの運営者、もしくは運営者が委託する第三者が運営するサービス、加えていずれの場合も本サービスが使用するドメイン({})を含むサービス", domain)
        ],
        "contact": contact_point
    });

    let mut resp = Json(terms_content).into_response();
    resp.headers_mut()
        .insert("Cache-Control", "s-maxage=300".parse().unwrap());
    resp
}

async fn get_robots_txt() -> impl IntoResponse {
    let robot_txt = "User-agent: *\nAllow: /\nDisallow: /auth-code\n";
    SJisResponseBuilder::new((robot_txt as &str).into())
        .client_ttl(60 * 60 * 24)
        .server_ttl(60 * 60 * 24)
        .content_type(SjisContentType::TextPlain)
        .build()
        .into_response()
}
