use auth::{auth_simple_header, get_check_auth, get_login, get_login_callback, get_logout};
use axum::{
    body::Body,
    extract::{MatchedPath, Request},
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
    Router, ServiceExt,
};
use chrono::Utc;
use eddist_core::{
    domain::{client_info::ClientInfo as CoreClientInfo, tinker::Tinker as CoreTinker},
    utils::is_prod,
};
use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};
use repository::admin_bbs_repository::{AdminBbsRepository, AdminBbsRepositoryImpl};
use serde::{Deserialize, Serialize};
use time::Duration;
use tokio::net::TcpListener;
use tower_layer::Layer;
use tower_sessions::{cookie::SameSite, Expiry, MemoryStore, SessionManagerLayer};
use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};
use utoipa::{IntoParams, OpenApi, ToSchema};
use uuid::Uuid;

use std::net::SocketAddr;
use tower_http::{
    normalize_path::NormalizePathLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing::info_span;

pub(crate) mod auth;
pub(crate) mod repository {
    pub mod admin_bbs_repository;
    pub mod admin_repository;
}
pub(crate) mod role;

async fn add_cors_header(req: Request<Body>, next: Next) -> Response {
    let mut res = next.run(req).await;
    res.headers_mut()
        .insert("Access-Control-Allow-Origin", HeaderValue::from_static("*"));
    res.headers_mut().insert(
        "Access-Control-Allow-Headers",
        HeaderValue::from_static("*"),
    );

    res
}

async fn ok() -> impl IntoResponse {
    StatusCode::OK
}

#[derive(Clone)]
struct AppState<T: AdminBbsRepository + Clone> {
    oauth2_client: oauth2::basic::BasicClient,
    repo: T,
    redis_conn: redis::aio::MultiplexedConnection,
}

#[tokio::main]
async fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if !args.is_empty() && args[0] == "--openapi" {
        let doc = ApiDoc::openapi().to_pretty_json().unwrap();
        std::fs::write("./eddist-admin/openapi.json", doc).unwrap();
        return;
    }

    let addr = SocketAddr::from(([0, 0, 0, 0], 8081));
    if !is_prod() {
        dotenvy::dotenv().unwrap();
    }

    let client = oauth2::basic::BasicClient::new(
        ClientId::new(std::env::var("EDDIST_ADMIN_CLIENT_ID").unwrap()),
        Some(ClientSecret::new(
            std::env::var("EDDIST_ADMIN_CLIENT_SECRET").unwrap(),
        )),
        AuthUrl::new(std::env::var("EDDIST_ADMIN_AUTH_URL").unwrap()).unwrap(),
        Some(TokenUrl::new(std::env::var("EDDIST_ADMIN_TOKEN_URL").unwrap()).unwrap()),
    )
    .set_redirect_uri(
        RedirectUrl::new(std::env::var("EDDIST_ADMIN_LOGIN_CALLBACK_URL").unwrap()).unwrap(),
    );

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_span_events(FmtSpan::CLOSE)
        .with_ansi(false)
        .init();

    let serve_dir = if is_prod() {
        "dist"
    } else {
        "eddist-admin/client/build/client"
    };
    let serve_dir = ServeDir::new(serve_dir)
        .not_found_service(ServeFile::new(format!("{serve_dir}/index.html")));

    let pool = sqlx::mysql::MySqlPool::connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();

    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(Duration::days(14)));

    let api_routes = Router::new()
        .route("/boards", get(bbs::get_boards))
        .route("/boards", post(bbs::create_board))
        .route("/boards/:boardKey", get(bbs::get_board))
        .route("/boards/:boardKey/threads", get(bbs::get_threads))
        .route("/boards/:boardKey/threads/:threadId", get(bbs::get_thread))
        .route(
            "/boards/:boardKey/threads/:threadId/responses",
            get(bbs::get_responses),
        )
        .route(
            "/boards/:boardKey/threads/:threadId/responses/:resId",
            patch(bbs::update_response),
        )
        .route(
            "/authed_tokens/:authedTokenId",
            delete(bbs::delete_authed_token),
        )
        .route("/ng_words", get(bbs::get_ng_words))
        .route("/ng_words", post(bbs::create_ng_word))
        .route("/ng_words/:ngWordId", delete(bbs::delete_ng_word))
        .route("/ng_words/:ngWordId", patch(bbs::update_ng_word));

    let state = AppState {
        oauth2_client: client,
        repo: AdminBbsRepositoryImpl::new(pool),
        redis_conn: redis::Client::open(std::env::var("REDIS_URL").unwrap())
            .unwrap()
            .get_multiplexed_tokio_connection()
            .await
            .unwrap(),
    };

    let app = Router::new()
        .route("/login", get(get_login))
        .route("/auth/check", get(get_check_auth))
        .route("/auth/logout", get(get_logout))
        .route("/auth/callback", get(get_login_callback))
        .nest(
            "/api",
            api_routes.layer(axum::middleware::from_fn_with_state(
                state.clone(),
                auth_simple_header,
            )),
        )
        .nest_service("/dist", serve_dir.clone())
        .fallback_service(serve_dir)
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &Request<_>| {
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
            }),
        )
        .layer(axum::middleware::from_fn(add_cors_header))
        .layer(session_layer)
        .with_state(state.clone());

    let app = NormalizePathLayer::trim_trailing_slash().layer(app);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, ServiceExt::<Request>::into_make_service(app))
        .await
        .unwrap();
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
struct Board {
    pub id: Uuid,
    pub name: String,
    pub board_key: String,
    pub default_name: String,
    pub thread_count: i64,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
struct CreateBoardInput {
    pub name: String,
    pub board_key: String,
    pub default_name: String,
    pub local_rule: String,
    pub base_thread_creation_span_sec: Option<usize>,
    pub base_response_creation_span_sec: Option<usize>,
    pub max_thread_name_byte_length: Option<usize>,
    pub max_author_name_byte_length: Option<usize>,
    pub max_email_byte_length: Option<usize>,
    pub max_response_body_byte_length: Option<usize>,
    pub max_response_body_lines: Option<usize>,
    pub threads_archive_cron: Option<String>,
    pub threads_archive_trigger_thread_count: Option<usize>,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
struct Thread {
    pub id: Uuid,
    pub board_id: Uuid,
    pub thread_number: u64,
    pub last_modified: chrono::DateTime<Utc>,
    pub sage_last_modified: chrono::DateTime<Utc>,
    pub title: String,
    pub authed_token_id: Uuid,
    pub metadent: String,
    pub response_count: u32,
    pub no_pool: bool,
    pub archived: bool,
    pub active: bool,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct Res {
    pub id: Uuid,
    pub author_name: Option<String>,
    pub mail: Option<String>,
    pub body: String,
    pub created_at: chrono::DateTime<Utc>,
    pub author_id: String,
    pub ip_addr: String,
    pub authed_token_id: Uuid,
    pub board_id: Uuid,
    pub thread_id: Uuid,
    pub is_abone: bool,
    pub client_info: ClientInfo,
    pub res_order: i32,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct ClientInfo {
    pub user_agent: String,
    pub asn_num: u32,
    pub ip_addr: String,
    pub tinker: Option<Tinker>,
}

impl From<CoreClientInfo> for ClientInfo {
    fn from(value: CoreClientInfo) -> Self {
        Self {
            user_agent: value.user_agent.to_string(),
            asn_num: value.asn_num,
            ip_addr: value.ip_addr().to_string(),
            tinker: value.tinker.as_deref().cloned().map(Tinker::from),
        }
    }
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct Tinker {
    authed_token: String,
    wrote_count: u32,
    created_thread_count: u32,
    level: u32,
    last_level_up_at: u64,
    last_wrote_at: u64,
    last_created_thread_at: Option<u64>,
}

impl From<CoreTinker> for Tinker {
    fn from(value: CoreTinker) -> Self {
        Self {
            authed_token: value.authed_token().to_string(),
            wrote_count: value.wrote_count(),
            created_thread_count: value.created_thread_count(),
            level: value.level(),
            last_level_up_at: value.last_level_up_at(),
            last_wrote_at: value.last_wrote_at(),
            last_created_thread_at: value.last_created_thread_at(),
        }
    }
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
struct UpdateResInput {
    author_name: Option<String>,
    mail: Option<String>,
    body: Option<String>,
    is_abone: Option<bool>,
}

#[derive(Debug, Clone, IntoParams, Serialize, Deserialize)]
struct DeleteAuthedTokenInput {
    using_origin_ip: bool,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
struct NgWord {
    id: Uuid,
    name: String,
    word: String,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
    board_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
struct CreationNgWordInput {
    name: String,
    word: String,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
struct UpdateNgWordInput {
    name: Option<String>,
    word: Option<String>,
    board_ids: Option<Vec<Uuid>>,
}

mod bbs {
    use axum::{
        extract::{Path, Query, State},
        response::Response,
        Json,
    };
    use eddist_core::domain::res::ResView;
    use uuid::Uuid;

    use crate::{
        repository::admin_bbs_repository::{AdminBbsRepository, AdminBbsRepositoryImpl},
        AppState, Board, CreateBoardInput, CreationNgWordInput, DeleteAuthedTokenInput, NgWord,
        Res, Thread, UpdateNgWordInput, UpdateResInput,
    };

    #[utoipa::path(
        get,
        path = "/boards/",
        responses(
            (status = 200, description = "List boards successfully", body = Vec<Board>),
        )
    )]
    pub async fn get_boards(
        State(state): State<AppState<AdminBbsRepositoryImpl>>,
    ) -> Json<Vec<Board>> {
        let boards = state.repo.get_boards_by_key(None).await.unwrap();
        boards.into()
    }

    #[utoipa::path(
        get,
        path = "/boards/{board_key}/",
        responses(
            (status = 200, description = "Get board successfully", body = Board),
            (status = 404, description = "Board not found"),
        ),
        params(
            ("board_key" = String, Path, description = "Board ID"),
        )
    )]
    pub async fn get_board(
        State(state): State<AppState<AdminBbsRepositoryImpl>>,
        Path(board_key): Path<String>,
    ) -> Response {
        let board = state
            .repo
            .get_boards_by_key(Some(vec![board_key]))
            .await
            .unwrap();
        let Some(board) = board.first() else {
            return Response::builder()
                .status(404)
                .body(axum::body::Body::empty())
                .unwrap();
        };

        Response::builder()
            .status(200)
            .body(serde_json::to_string(&board).unwrap().into())
            .unwrap()
    }

    #[utoipa::path(
        post,
        path = "/boards/",
        responses(
            (status = 200, description = "Create board successfully", body = CreateBoardInput),
        ),
        request_body = CreateBoardInput
    )]
    pub async fn create_board(
        State(state): State<AppState<AdminBbsRepositoryImpl>>,
        Json(body): Json<CreateBoardInput>,
    ) -> Response {
        let board = state.repo.create_board(body).await.unwrap();

        Response::builder()
            .status(200)
            .body(serde_json::to_string(&board).unwrap().into())
            .unwrap()
    }

    #[utoipa::path(
        get,
        path = "/boards/{board_key}/threads/",
        responses(
            (status = 200, description = "List threads successfully", body = Vec<Thread>),
        )
    )]
    pub async fn get_threads(
        State(state): State<AppState<AdminBbsRepositoryImpl>>,
        Path(board_key): Path<String>,
    ) -> Json<Vec<Thread>> {
        let threads = state
            .repo
            .get_threads_by_thread_id(&board_key, None)
            .await
            .unwrap();

        threads.into()
    }

    #[utoipa::path(
        get,
        path = "/boards/{board_key}/threads/{thread_id}/",
        responses(
            (status = 200, description = "Get thread successfully", body = Thread),
            (status = 404, description = "Thread not found"),
        ),
        params(
            ("board_key" = String, Path, description = "Board ID"),
            ("thread_id" = u64, Path, description = "Thread ID"),
        )
    )]
    pub async fn get_thread(
        State(state): State<AppState<AdminBbsRepositoryImpl>>,
        Path((board_key, thread_id)): Path<(String, u64)>,
    ) -> Response {
        let thread = state
            .repo
            .get_threads_by_thread_id(&board_key, Some(vec![thread_id]))
            .await
            .unwrap();

        let Some(thread) = thread.first() else {
            return Response::builder()
                .status(404)
                .body(axum::body::Body::empty())
                .unwrap();
        };

        Response::builder()
            .status(200)
            .body(serde_json::to_string(&thread).unwrap().into())
            .unwrap()
    }

    #[utoipa::path(
        get,
        path = "/boards/{board_key}/threads/{thread_id}/responses/",
        responses(
            (status = 200, description = "List responses successfully", body = Vec<Res>),
            (status = 404, description = "Thread not found"),
        ),
        params(
            ("thread_id" = u64, Path, description = "Thread ID"),
        )
    )]
    pub async fn get_responses(
        State(state): State<AppState<AdminBbsRepositoryImpl>>,
        Path((board_key, thread_id)): Path<(String, u64)>,
    ) -> Json<Vec<Res>> {
        let responses = state
            .repo
            .get_reses_by_thread_id(&board_key, thread_id)
            .await
            .unwrap();

        responses.into()
    }

    #[utoipa::path(
        patch,
        path = "/boards/{board_key}/threads/{thread_id}/responses/{res_id}/",
        responses(
            (status = 200, description = "Update response successfully", body = Res),
        ),
        params(
            ("board_key" = String, Path, description = "Board ID"),
            ("thread_id" = u64, Path, description = "Thread ID"),
            ("res_id" = Uuid, Path, description = "Response ID"),
        ),
        request_body = UpdateResInput
    )]
    pub async fn update_response(
        State(state): State<AppState<AdminBbsRepositoryImpl>>,
        Path((_a, _aa, res_id)): Path<(String, u64, Uuid)>,
        Json(body): Json<UpdateResInput>,
    ) -> Response {
        let (res, default_name, board_key, thread_number, thread_title) =
            state.repo.get_res(res_id).await.unwrap();
        let updated_res = state
            .repo
            .update_res(
                res_id,
                body.author_name.clone(),
                body.mail.clone(),
                body.body.clone(),
                body.is_abone,
            )
            .await
            .unwrap();
        let author_name = if let Some(author_name) = body.author_name {
            author_name
        } else {
            res.author_name.unwrap_or(default_name.clone())
        };
        let mail = if let Some(mail) = body.mail {
            mail
        } else {
            res.mail.unwrap_or_default()
        };
        let is_abone = if let Some(is_abone) = body.is_abone {
            is_abone
        } else {
            res.is_abone
        };
        let res_body = if let Some(body) = body.body {
            body
        } else {
            res.body
        };

        let res_view = ResView {
            author_name,
            mail,
            body: res_body,
            created_at: res.created_at,
            author_id: res.author_id,
            is_abone,
        };

        let res_view = res_view.get_sjis_bytes(&default_name, thread_title.as_deref());
        let mut conn = state.redis_conn;
        let _ = conn
            .send_packed_command(&redis::Cmd::lset(
                format!("threads:{}:{}", board_key, thread_number),
                res.res_order as isize - 1,
                res_view.get_inner(),
            ))
            .await;

        Response::builder()
            .status(200)
            .body(serde_json::to_string(&updated_res).unwrap().into())
            .unwrap()
    }

    #[utoipa::path(
        delete,
        path = "/authed_tokens/{authed_token_id}/",
        responses(
            (status = 200, description = "Delete authed token successfully"),
        ),
        params(
            ("authed_token_id" = Uuid, Path, description = "Authed token ID"),
            DeleteAuthedTokenInput
        ),
    )]
    pub async fn delete_authed_token(
        State(state): State<AppState<AdminBbsRepositoryImpl>>,
        Path(authed_token_id): Path<Uuid>,
        Query(DeleteAuthedTokenInput { using_origin_ip }): Query<DeleteAuthedTokenInput>,
    ) -> Response {
        if using_origin_ip {
            state
                .repo
                .delete_authed_token(authed_token_id)
                .await
                .unwrap();
        } else {
            state
                .repo
                .delete_authed_token_by_origin_ip(authed_token_id)
                .await
                .unwrap();
        }

        Response::builder()
            .status(200)
            .body(axum::body::Body::empty())
            .unwrap()
    }

    #[utoipa::path(
        get,
        path = "/ng_words/",
        responses(
            (status = 200, description = "List ng words successfully", body = Vec<NgWord>),
        )
    )]
    pub async fn get_ng_words(
        State(state): State<AppState<AdminBbsRepositoryImpl>>,
    ) -> Json<Vec<NgWord>> {
        let ng_words = state.repo.get_ng_words().await.unwrap();
        ng_words.into()
    }

    #[utoipa::path(
        post,
        path = "/ng_words/",
        responses(
            (status = 200, description = "Create ng word successfully", body = NgWord),
        ),
        request_body = CreationNgWordInput
    )]
    pub async fn create_ng_word(
        State(state): State<AppState<AdminBbsRepositoryImpl>>,
        Json(body): Json<CreationNgWordInput>,
    ) -> Response {
        let ng_word = state
            .repo
            .create_ng_word(&body.name, &body.word)
            .await
            .unwrap();

        Response::builder()
            .status(200)
            .body(serde_json::to_string(&ng_word).unwrap().into())
            .unwrap()
    }

    #[utoipa::path(
        patch,
        path = "/ng_words/{ng_word_id}/",
        responses(
            (status = 200, description = "Update ng word successfully", body = NgWord),
        ),
        params(
            ("ng_word_id" = Uuid, Path, description = "NG word ID"),
        ),
        request_body = UpdateNgWordInput
    )]
    pub async fn update_ng_word(
        State(state): State<AppState<AdminBbsRepositoryImpl>>,
        Path(ng_word_id): Path<Uuid>,
        Json(body): Json<UpdateNgWordInput>,
    ) -> Response {
        let ng_word = state
            .repo
            .update_ng_word(
                ng_word_id,
                body.name.as_deref(),
                body.word.as_deref(),
                body.board_ids,
            )
            .await
            .unwrap();

        Response::builder()
            .status(200)
            .body(serde_json::to_string(&ng_word).unwrap().into())
            .unwrap()
    }

    #[utoipa::path(
        delete,
        path = "/ng_words/{ng_word_id}/",
        responses(
            (status = 200, description = "Delete ng word successfully"),
        ),
        params(
            ("ng_word_id" = Uuid, Path, description = "NG word ID"),
        ),
    )]
    pub async fn delete_ng_word(
        State(state): State<AppState<AdminBbsRepositoryImpl>>,
        Path(ng_word_id): Path<Uuid>,
    ) -> Response {
        state.repo.delete_ng_word(ng_word_id).await.unwrap();

        Response::builder()
            .status(200)
            .body(axum::body::Body::empty())
            .unwrap()
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(
        bbs::get_boards,
        bbs::get_board,
        bbs::create_board,
        bbs::get_threads,
        bbs::get_thread,
        bbs::get_responses,
        bbs::update_response,
        bbs::delete_authed_token,
        bbs::get_ng_words,
        bbs::create_ng_word,
        bbs::update_ng_word,
        bbs::delete_ng_word,
    ),
    components(schemas(
        Board,
        Thread,
        Res,
        ClientInfo,
        Tinker,
        UpdateResInput,
        NgWord,
        CreationNgWordInput,
        UpdateNgWordInput,
        CreateBoardInput,
    ))
)]
struct ApiDoc;
