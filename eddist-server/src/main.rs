use std::{convert::Infallible, env, time::Duration};

use axum::{
    body::{Body, Bytes},
    extract::{MatchedPath, Path, State},
    http::{HeaderMap, Request, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Form, Router,
};
use axum_extra::extract::CookieJar;
use base64::Engine;
use eddist_core::domain::tinker::Tinker;
use error::{BbsCgiError, InsufficientParamType, InvalidParamType};
use hyper::{server::conn::http1, service::service_fn};
use hyper_util::rt::{TokioIo, TokioTimer};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Validation};
use repositories::bbs_repository::BbsRepositoryImpl;
use serde::Deserialize;
use services::{
    auth_with_code_service::{AuthWithCodeServiceInput, AuthWithCodeServiceOutput},
    board_info_service::{BoardInfoServiceInput, BoardInfoServiceOutput},
    res_creation_service::{ResCreationServiceInput, ResCreationServiceOutput},
    thread_creation_service::{TheradCreationServiceInput, TheradCreationServiceOutput},
    thread_list_service::BoardKey,
    thread_retrieval_service::ThreadRetrievalServiceInput,
    AppService, AppServiceContainer, BbsCgiService,
};
use shiftjis::{
    shift_jis_url_encodeded_body_to_vec, SJisResponseBuilder, SJisStr, SjisContentType,
};
use sqlx::mysql::MySqlPoolOptions;
use tokio::net::TcpListener;
use tower_http::{
    classify::ServerErrorsFailureClass,
    compression::{
        predicate::{NotForContentType, SizeAbove},
        CompressionLayer, Predicate,
    },
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::{info_span, Span};
use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};

mod shiftjis;
mod repositories {
    pub(crate) mod bbs_repository;
}
mod domain {
    pub(crate) mod service {
        pub mod bbscgi_auth_service;
        pub mod ng_word_reading_service;
    }

    pub(crate) mod authed_token;
    pub(crate) mod board;
    pub(crate) mod metadent;
    pub(crate) mod ng_word;
    pub(crate) mod res;
    pub(crate) mod res_core;
    pub(crate) mod res_view;
    pub(crate) mod thread;
    pub(crate) mod thread_list;
    pub(crate) mod thread_res_list;

    pub(crate) mod utils;
}
mod error;
mod services;

#[derive(Debug, Clone)]
struct AppState {
    services: AppServiceContainer<BbsRepositoryImpl>,
    tinker_secret: Vec<u8>,
}

impl AppState {
    pub fn get_container(&self) -> &AppServiceContainer<BbsRepositoryImpl> {
        &self.services
    }

    pub fn tinker_secret(&self) -> &[u8] {
        &self.tinker_secret
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if !matches!(env::var("RUST_ENV").as_deref(), Ok("prod" | "production")) {
        dotenvy::dotenv()?;
    }

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_span_events(FmtSpan::CLOSE)
        .with_ansi(false)
        .init();

    let client = redis::Client::open(env::var("REDIS_URL").unwrap())?;
    let con = client.get_multiplexed_tokio_connection().await?;

    let pool = MySqlPoolOptions::new()
        .max_connections(8)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&env::var("DATABASE_URL")?)
        .await?;

    let tinker_secret = env::var("TINKER_SECRET").unwrap();
    let tinker_secret = base64::engine::general_purpose::STANDARD
        .decode(tinker_secret.as_bytes())
        .unwrap();

    let app_state = AppState {
        services: AppServiceContainer::new(BbsRepositoryImpl::new(pool), con),
        tinker_secret,
    };

    log::info!("Start application server with 0.0.0.0:8080");

    let app = Router::new()
        .route("/", get(get_home))
        .route("/health-check", get(health_check))
        .route("/auth-code", get(get_auth_code).post(post_auth_code))
        .route("/test/bbs.cgi", post(post_bbs_cgi))
        .route("/:boardKey/subject.txt", get(get_subject_txt))
        .route("/:boardKey/head.txt", get(get_subject_txt))
        .route("/:boardKey/SETTING.TXT", get(get_setting_txt))
        .route("/:boardKey/dat/:threadId", get(get_dat_txt))
        .route("/:boardKey/kako/:th4/:th5/:threadId", get(get_kako_dat_txt))
        .with_state(app_state)
        .layer(TimeoutLayer::new(Duration::from_secs(10)))
        .layer(
            CompressionLayer::new().gzip(true).br(false).compress_when(
                SizeAbove::new(1024)
                    .and(NotForContentType::GRPC)
                    .and(NotForContentType::IMAGES),
            ),
        )
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
        );
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

async fn get_home(headers: HeaderMap) -> String {
    format!("{headers:?}")
}

async fn get_subject_txt(
    State(state): State<AppState>,
    Path(board_key): Path<String>,
) -> impl IntoResponse {
    let svc = state.get_container().thread_list();
    let threads = svc.execute(BoardKey(board_key)).await.unwrap();

    SJisResponseBuilder::new(SJisStr::from_unchecked_vec(threads.get_sjis_thread_list()))
        .content_type(SjisContentType::TextPlain)
        .client_ttl(5)
        .server_ttl(1)
        .build()
        .into_response()
}

async fn get_dat_txt(
    State(state): State<AppState>,
    Path((board_key, thread_id_with_dat)): Path<(String, String)>,
) -> Response {
    if thread_id_with_dat.len() != 14 {
        panic!("invalid dat")
    }
    let thread_number = thread_id_with_dat.replace(".dat", "");

    let svc = state.get_container().thread_retrival();
    let result = svc
        .execute(ThreadRetrievalServiceInput {
            board_key,
            thread_number: thread_number.parse::<u64>().unwrap(),
        })
        .await
        .unwrap();

    SJisResponseBuilder::new(SJisStr::from_unchecked_vec(result.raw()))
        .content_type(SjisContentType::TextPlain)
        .client_ttl(5)
        .server_ttl(1)
        .build()
        .into_response()
}

async fn get_kako_dat_txt(
    State(state): State<AppState>,
    Path((board_key, _, _, thread_id_with_dat)): Path<(String, String, String, String)>,
) -> Response {
    if thread_id_with_dat.len() != 14 {
        panic!("invalid dat")
    }
    let thread_number = thread_id_with_dat.replace(".dat", "");

    let svc = state.get_container().thread_retrival();
    let result = svc
        .execute(ThreadRetrievalServiceInput {
            board_key,
            thread_number: thread_number.parse::<u64>().unwrap(),
        })
        .await
        .unwrap();

    SJisResponseBuilder::new(SJisStr::from_unchecked_vec(result.raw()))
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
    } = state
        .services
        .board_info()
        .execute(BoardInfoServiceInput { board_key })
        .await
        .unwrap();
    let setting_txt = format!(
        "{board_key}@{board_key}
BBS_TITLE={name}
BBS_TITLE_ORIG={name}
BBS_NONAME_NAME={default_name}"
    );

    SJisResponseBuilder::new((&setting_txt as &str).into())
        .client_ttl(120)
        .server_ttl(300)
        .content_type(SjisContentType::TextPlain)
        .build()
        .into_response()
}

// NOTE: this system will be changed in the future
async fn get_auth_code() -> impl IntoResponse {
    Html(
        r#"<html>
<head>
    <title>コード認証画面</title>
    <meta charset="utf-8">
    <script src="https://challenges.cloudflare.com/turnstile/v0/api.js" async defer></script>
<body>
    <p>認証を進めるために、事前に書き込みを行い6桁の認証コードを取得してください</p>
    <form action="/auth-code" method="POST">
        <!--<div class="cf-turnstile" data-sitekey="{cf_site_key}" data-theme="light"></div>-->
        <input type="number" name="auth-code" placeholder="6桁の認証コード">
        <input type="submit" value="Submit">
    </form>
</body>
</html>"#
            .to_string(),
    )
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct PostAuthCodeForm {
    auth_code: u32,
}

async fn post_auth_code(
    headers: HeaderMap,
    State(state): State<AppState>,
    Form(form): Form<PostAuthCodeForm>,
) -> impl IntoResponse {
    let AuthWithCodeServiceOutput { token } = state
        .services
        .auth_with_code()
        .execute(AuthWithCodeServiceInput {
            code: form.auth_code.to_string(),
            origin_ip: get_origin_ip(&headers).to_string(),
            user_agent: get_ua(&headers).to_string(),
        })
        .await
        .unwrap();

    Html(r##"<html>
<head>
    <title>認証成功 - Successful</title>
    <meta charset="utf-8">
</head>
<body>
    <p>認証に成功しました</p>
    <p>再びそのまま書き込みを行うか、メール欄に以下を貼り付けてください（#以降の内容は書き込み時に消えます、いくつかのブラウザでは貼り付けなくても書けます）</p>
    <input type="text" value="#{token}" onfocus="this.select();" style="width: 50rem;"></input>
</body>
</html>"##.replace("{token}", &token))
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
        .get("tinker")
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
            Ok(TheradCreationServiceOutput { tinker }) => tinker,
            Err(e) => return e.into_response(),
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
            Err(e) => return e.into_response(),
        }
    };

    SJisResponseBuilder::new(SJisStr::from(
        r#"<html><!-- 2ch_X:true -->
<head>
    <meta http-equiv="Content-Type" content="text/html; charset=x-sjis">
    <title>書きこみました</title>
</head>
<body>書きこみました</body>
</html>\n"#,
    ))
    .content_type(SjisContentType::TextHtml)
    .add_set_cookie(
        "tinker".to_string(),
        jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &tinker,
            &EncodingKey::from_secret(state.tinker_secret()),
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

fn get_origin_ip(headers: &HeaderMap) -> &str {
    headers
        .get("Cf-Connecting-IP")
        .or_else(|| headers.get("X-Forwarded-For"))
        .map(|x| x.to_str())
        .unwrap_or(Ok("localhost")) // FIXME: for development only
        .unwrap()
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

    headers
        .get(header_name)
        .map(|x| x.to_str())
        .unwrap_or(Ok("0"))
        .unwrap()
        .parse::<u32>()
        .unwrap()
}

fn get_tinker(tinker: &str, secret: &[u8]) -> Option<Tinker> {
    let tinker = jsonwebtoken::decode::<Tinker>(
        tinker,
        &DecodingKey::from_secret(secret),
        &Validation::new(Algorithm::HS256),
    )
    .ok()?;

    Some(tinker.claims)
}
