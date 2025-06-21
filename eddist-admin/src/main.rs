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
    tracing::init_tracing,
    utils::is_prod,
};
use oauth2::{AuthUrl, ClientId, ClientSecret, EndpointNotSet, EndpointSet, RedirectUrl, TokenUrl};
use repository::{
    admin_archive_repository::{
        AdminArchiveRepository, AdminArchiveRepositoryImpl, ArchivedAdminRes, ArchivedAdminThread,
        ArchivedRes, ArchivedResUpdate, ArchivedThread,
    },
    admin_bbs_repository::{AdminBbsRepository, AdminBbsRepositoryImpl},
    admin_user_repository::{AdminUserRepository, AdminUserRepositoryImpl},
    authed_token_repository::{AuthedTokenRepository, AuthedTokenRepositoryImpl},
    cap_repository::{CapRepository, CapRepositoryImpl},
    ngword_repository::{NgWordRepository, NgWordRepositoryImpl},
};
use s3::creds::Credentials;
use serde::{Deserialize, Serialize};
use time::Duration;
use tokio::net::TcpListener;
use tower_layer::Layer;
use tower_sessions::{cookie::SameSite, Expiry, MemoryStore, SessionManagerLayer};
use utoipa::{IntoParams, OpenApi, ToSchema};
use uuid::Uuid;

use std::{env, net::SocketAddr};
use tower_http::{
    normalize_path::NormalizePathLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing::info_span;

pub(crate) mod auth;
pub(crate) mod repository {
    pub mod admin_archive_repository;
    pub mod admin_bbs_repository;
    pub mod admin_repository;
    pub mod admin_user_repository;
    pub mod authed_token_repository;
    pub mod cap_repository;
    pub mod ngword_repository;
}
pub(crate) mod role;

async fn add_some_header(req: Request<Body>, next: Next) -> Response {
    let mut res = next.run(req).await;

    // Cache-Control: private
    res.headers_mut()
        .insert("Cache-Control", HeaderValue::from_static("private"));

    // if local env
    if matches!(
        std::env::var("RUST_ENV").as_deref(),
        Ok("prod" | "production")
    ) {
        return res;
    }
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
struct AppState<
    T: AdminBbsRepository + Clone,
    R: AdminArchiveRepository + Clone,
    N: NgWordRepository + Clone,
    A: AuthedTokenRepository + Clone,
    C: CapRepository + Clone,
    U: AdminUserRepository + Clone,
> {
    oauth2_client: oauth2::basic::BasicClient<
        EndpointSet,
        EndpointNotSet,
        EndpointNotSet,
        EndpointNotSet,
        EndpointSet,
    >,
    admin_bbs_repo: T,
    ng_word_repo: N,
    admin_archive_repo: R,
    authed_token_repo: A,
    cap_repo: C,
    user_repo: U,
    redis_conn: redis::aio::ConnectionManager,
}

type DefaultAppState = AppState<
    AdminBbsRepositoryImpl,
    AdminArchiveRepositoryImpl,
    NgWordRepositoryImpl,
    AuthedTokenRepositoryImpl,
    CapRepositoryImpl,
    AdminUserRepositoryImpl,
>;

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

    let client = oauth2::basic::BasicClient::new(ClientId::new(
        std::env::var("EDDIST_ADMIN_CLIENT_ID").unwrap(),
    ))
    .set_client_secret(ClientSecret::new(
        std::env::var("EDDIST_ADMIN_CLIENT_SECRET").unwrap(),
    ))
    .set_auth_uri(AuthUrl::new(std::env::var("EDDIST_ADMIN_AUTH_URL").unwrap()).unwrap())
    .set_token_uri(TokenUrl::new(std::env::var("EDDIST_ADMIN_TOKEN_URL").unwrap()).unwrap())
    .set_redirect_uri(
        RedirectUrl::new(std::env::var("EDDIST_ADMIN_LOGIN_CALLBACK_URL").unwrap()).unwrap(),
    );

    init_tracing();

    let serve_dir = if is_prod() {
        "dist"
    } else {
        "eddist-admin/client/build/client"
    };
    let serve_dir = ServeDir::new(serve_dir)
        .not_found_service(ServeFile::new(format!("{serve_dir}/index.html")));

    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .after_connect(|conn, _| {
            use sqlx::Executor;

            Box::pin(async move {
                conn.execute(
                    "SET SESSION sql_mode = CONCAT(@@sql_mode, ',TIME_TRUNCATE_FRACTIONAL')",
                )
                .await
                .unwrap();
                log::info!("Set TIME_TRUNCATE_FRACTIONAL mode");
                Ok(())
            })
        })
        .connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();

    let s3_client = s3::bucket::Bucket::new(
        env::var("S3_BUCKET_NAME").unwrap().trim(),
        s3::Region::R2 {
            account_id: env::var("R2_ACCOUNT_ID").unwrap().trim().to_string(),
        },
        Credentials::new(
            Some(env::var("S3_ACCESS_KEY").unwrap().trim()),
            Some(env::var("S3_ACCESS_SECRET_KEY").unwrap().trim()),
            None,
            None,
            None,
        )
        .unwrap(),
    )
    .unwrap();

    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(Duration::days(14)));

    let api_routes = Router::new()
        .route("/boards", get(bbs::get_boards))
        .route("/boards", post(bbs::create_board))
        .route("/boards/{boardKey}", get(bbs::get_board))
        .route("/boards/{boardKey}/info", get(bbs::get_board_info))
        .route("/boards/{boardKey}", patch(bbs::edit_board))
        .route("/boards/{boardKey}/threads", get(bbs::get_threads))
        .route(
            "/boards/{boardKey}/threads/{threadId}",
            get(bbs::get_thread),
        )
        .route(
            "/boards/{boardKey}/threads/{threadId}/responses",
            get(bbs::get_responses),
        )
        .route(
            "/boards/{boardKey}/archives",
            get(bbs::get_archived_threads),
        )
        .route(
            "/boards/{boardKey}/archives/{threadId}",
            get(bbs::get_archived_thread),
        )
        .route(
            "/boards/{boardKey}/archives/{threadId}/responses",
            get(bbs::get_archived_responses),
        )
        .route(
            "/boards/{boardKey}/threads/{threadId}/responses/{resId}",
            patch(bbs::update_response),
        )
        .route(
            "/boards/{boardKey}/dat-archives/{threadNumber}",
            get(bbs::get_dat_archived_thread),
        )
        .route(
            "/boards/{boardKey}/admin-dat-archives/{threadNumber}",
            get(bbs::get_admin_dat_archived_thread),
        )
        .route(
            "/boards/{boardKey}/dat-archives/{threadNumber}/responses",
            patch(bbs::update_archived_res),
        )
        .route(
            "/boards/{boardKey}/dat-archives/{threadNumber}/responses/{resOrder}",
            delete(bbs::delete_archived_res),
        )
        .route(
            "/boards/{boardKey}/dat-archives/{threadNumber}",
            delete(bbs::delete_archived_thread),
        )
        .route(
            "/boards/{boardKey}/threads-compaction/",
            post(bbs::threads_compaction),
        )
        .route("/authed_tokens/{authedTokenId}", get(bbs::get_authed_token))
        .route(
            "/authed_tokens/{authedTokenId}",
            delete(bbs::delete_authed_token),
        )
        .route("/ng_words", get(bbs::get_ng_words))
        .route("/ng_words", post(bbs::create_ng_word))
        .route("/ng_words/{ngWordId}", delete(bbs::delete_ng_word))
        .route("/ng_words/{ngWordId}", patch(bbs::update_ng_word))
        .route("/caps", get(bbs::get_caps))
        .route("/caps", post(bbs::create_cap))
        .route("/caps/{capId}", delete(bbs::delete_cap))
        .route("/caps/{capId}", patch(bbs::update_cap))
        .route("/users/search", get(bbs::search_users))
        .route("/users/{userId}/status", patch(bbs::update_user_status));

    let state = AppState {
        oauth2_client: client,
        admin_bbs_repo: AdminBbsRepositoryImpl::new(pool.clone()),
        ng_word_repo: NgWordRepositoryImpl::new(pool.clone()),
        redis_conn: redis::Client::open(std::env::var("REDIS_URL").unwrap())
            .unwrap()
            .get_connection_manager()
            .await
            .unwrap(),
        admin_archive_repo: AdminArchiveRepositoryImpl::new(*s3_client),
        authed_token_repo: AuthedTokenRepositoryImpl::new(pool.clone()),
        cap_repo: CapRepositoryImpl::new(pool.clone()),
        user_repo: AdminUserRepositoryImpl::new(pool),
    };

    let app = Router::new()
        .route("/health-check", get(ok))
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
        .layer(axum::middleware::from_fn(add_some_header))
        .layer(session_layer)
        .with_state(state.clone());

    let app = NormalizePathLayer::trim_trailing_slash().layer(app);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, ServiceExt::<Request>::into_make_service(app))
        .await
        .unwrap();
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct AuthedToken {
    id: Uuid,
    token: String,
    origin_ip: String,
    reduced_origin_ip: String,
    writing_ua: String,
    authed_ua: Option<String>,
    created_at: chrono::NaiveDateTime,
    authed_at: Option<chrono::NaiveDateTime>,
    validity: bool,
    last_wrote_at: Option<chrono::NaiveDateTime>,
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
struct BoardInfo {
    pub local_rules: String,
    pub base_thread_creation_span_sec: usize,
    pub base_response_creation_span_sec: usize,
    pub max_thread_name_byte_length: usize,
    pub max_author_name_byte_length: usize,
    pub max_email_byte_length: usize,
    pub max_response_body_byte_length: usize,
    pub max_response_body_lines: usize,
    pub threads_archive_cron: Option<String>,
    pub threads_archive_trigger_thread_count: Option<usize>,
    pub read_only: bool,
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
struct EditBoardInput {
    pub name: Option<String>,
    pub default_name: Option<String>,
    pub local_rule: Option<String>,
    pub base_thread_creation_span_sec: Option<usize>,
    pub base_response_creation_span_sec: Option<usize>,
    pub max_thread_name_byte_length: Option<usize>,
    pub max_author_name_byte_length: Option<usize>,
    pub max_email_byte_length: Option<usize>,
    pub max_response_body_byte_length: Option<usize>,
    pub max_response_body_lines: Option<usize>,
    pub threads_archive_cron: Option<String>,
    pub threads_archive_trigger_thread_count: Option<usize>,
    pub read_only: Option<bool>,
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
struct Cap {
    id: Uuid,
    name: String,
    description: String,
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

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
struct CreationCapInput {
    name: String,
    description: String,
    password: String,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
struct UpdateCapInput {
    name: Option<String>,
    description: Option<String>,
    password: Option<String>,
    board_ids: Option<Vec<Uuid>>,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct ThreadCompactionInput {
    target_count: u32,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct User {
    id: Uuid,
    user_name: String,
    enabled: bool,
    idp_bindings: Vec<UserIdpBinding>,
    authed_token_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct UserIdpBinding {
    id: Uuid,
    user_id: Uuid,
    idp_name: String,
    idp_sub: String,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct UserStatusUpdateInput {
    enabled: bool,
}

mod bbs {
    use axum::{
        extract::{Path, Query, State},
        response::Response,
        Json,
    };
    use chrono::{TimeZone, Utc};
    use eddist_core::domain::{board::validate_board_key, res::ResView};
    use serde::{Deserialize, Serialize};
    use utoipa::IntoParams;
    use uuid::Uuid;

    use crate::{
        repository::{
            admin_archive_repository::{
                AdminArchiveRepository, ArchivedAdminThread, ArchivedResUpdate, ArchivedThread,
            },
            admin_bbs_repository::AdminBbsRepository,
            admin_user_repository::AdminUserRepository,
            authed_token_repository::AuthedTokenRepository,
            cap_repository::CapRepository,
            ngword_repository::NgWordRepository,
        },
        AuthedToken, Board, BoardInfo, Cap, CreateBoardInput, CreationCapInput,
        CreationNgWordInput, DefaultAppState, DeleteAuthedTokenInput, EditBoardInput, NgWord, Res,
        Thread, ThreadCompactionInput, UpdateCapInput, UpdateNgWordInput, UpdateResInput, User,
        UserStatusUpdateInput,
    };

    #[utoipa::path(
        get,
        path = "/boards/",
        responses(
            (status = 200, description = "List boards successfully", body = Vec<Board>),
        )
    )]
    pub async fn get_boards(State(state): State<DefaultAppState>) -> Json<Vec<Board>> {
        let boards = state.admin_bbs_repo.get_boards_by_key(None).await.unwrap();
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
        State(state): State<DefaultAppState>,
        Path(board_key): Path<String>,
    ) -> Response {
        let board = state
            .admin_bbs_repo
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
        get,
        path = "/boards/{board_key}/info/",
        responses(
            (status = 200, description = "Get board info successfully", body = BoardInfo),
            (status = 404, description = "Board not found"),
        ),
        params(
            ("board_key" = String, Path, description = "Board ID"),
        ))
    ]
    pub async fn get_board_info(
        State(state): State<DefaultAppState>,
        Path(board_key): Path<String>,
    ) -> Response {
        let board = state
            .admin_bbs_repo
            .get_boards_by_key(Some(vec![board_key]))
            .await
            .unwrap();
        let Some(board) = board.first() else {
            return Response::builder()
                .status(404)
                .body(axum::body::Body::empty())
                .unwrap();
        };

        let board_info = state.admin_bbs_repo.get_board_info(board.id).await.unwrap();

        Response::builder()
            .status(200)
            .body(serde_json::to_string(&board_info).unwrap().into())
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
        State(state): State<DefaultAppState>,
        Json(body): Json<CreateBoardInput>,
    ) -> Response {
        if validate_board_key(&body.board_key).is_err() {
            return Response::builder()
                .status(400)
                .body("board_key must be ascii lower alphabetic or numeric".into())
                .unwrap();
        }

        let board = state.admin_bbs_repo.create_board(body).await.unwrap();

        Response::builder()
            .status(200)
            .body(serde_json::to_string(&board).unwrap().into())
            .unwrap()
    }

    #[utoipa::path(
        patch,
        path = "/boards/{board_key}/",
        responses(
            (status = 200, description = "Edit board successfully", body = Board),
        ),
        params(
            ("board_key" = Uuid, Path, description = "Board Key"),
        ),
        request_body = EditBoardInput
    )]
    pub async fn edit_board(
        State(state): State<DefaultAppState>,
        Path(board_key): Path<String>,
        Json(body): Json<EditBoardInput>,
    ) -> Response {
        let board = state
            .admin_bbs_repo
            .edit_board(&board_key, body)
            .await
            .unwrap();

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
        State(state): State<DefaultAppState>,
        Path(board_key): Path<String>,
    ) -> Json<Vec<Thread>> {
        let threads = state
            .admin_bbs_repo
            .get_threads_by_thread_id(&board_key, None)
            .await
            .unwrap();

        threads.into()
    }

    #[derive(Debug, Clone, Serialize, Deserialize, IntoParams)]
    pub struct GetArchivedThreadsQuery {
        keyword: Option<String>,
        start: Option<u64>,
        end: Option<u64>,
        page: Option<u64>,
        limit: Option<u64>,
    }

    #[utoipa::path(
        get,
        path = "/boards/{board_key}/archives/",
        params(
            ("board_key" = String, Path, description = "Board ID"),
            GetArchivedThreadsQuery
        ),
        responses(
            (status = 200, description = "List threads successfully", body = Vec<Thread>),
        )
    )]
    pub async fn get_archived_threads(
        State(state): State<DefaultAppState>,
        Path(board_key): Path<String>,
        Query(GetArchivedThreadsQuery {
            keyword,
            start,
            end,
            page,
            limit,
        }): Query<GetArchivedThreadsQuery>,
    ) -> Json<Vec<Thread>> {
        let threads = state
            .admin_bbs_repo
            .get_archived_threads_by_filter(
                &board_key,
                keyword.as_deref(),
                (
                    start.map(|x| Utc.timestamp_opt(x as i64, 0).unwrap().to_utc()),
                    end.map(|x| Utc.timestamp_opt(x as i64, 0).unwrap().to_utc()),
                ),
                page.unwrap_or(0),
                limit.unwrap_or(20),
            )
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
        State(state): State<DefaultAppState>,
        Path((board_key, thread_id)): Path<(String, u64)>,
    ) -> Response {
        let thread = state
            .admin_bbs_repo
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
        path = "/boards/{board_key}/archives/{thread_id}/",
        responses(
            (status = 200, description = "Get thread successfully", body = Thread),
            (status = 404, description = "Thread not found"),
        ),
        params(
            ("board_key" = String, Path, description = "Board ID"),
            ("thread_id" = u64, Path, description = "Thread ID"),
        )
    )]
    pub async fn get_archived_thread(
        State(state): State<DefaultAppState>,
        Path((board_key, thread_id)): Path<(String, u64)>,
    ) -> Response {
        let thread = state
            .admin_bbs_repo
            .get_archived_threads_by_thread_id(&board_key, Some(vec![thread_id]))
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
        State(state): State<DefaultAppState>,
        Path((board_key, thread_id)): Path<(String, u64)>,
    ) -> Json<Vec<Res>> {
        let responses = state
            .admin_bbs_repo
            .get_reses_by_thread_id(&board_key, thread_id)
            .await
            .unwrap();

        responses.into()
    }

    #[utoipa::path(
        get,
        path = "/boards/{board_key}/archives/{thread_id}/responses/",
        responses(
            (status = 200, description = "List responses successfully", body = Vec<Res>),
            (status = 404, description = "Thread not found"),
        ),
        params(
            ("thread_id" = u64, Path, description = "Thread ID"),
        )
    )]
    pub async fn get_archived_responses(
        State(state): State<DefaultAppState>,
        Path((board_key, thread_id)): Path<(String, u64)>,
    ) -> Json<Vec<Res>> {
        let responses = state
            .admin_bbs_repo
            .get_archived_reses_by_thread_id(&board_key, thread_id)
            .await
            .unwrap();

        responses.into()
    }

    #[utoipa::path(
        get,
        path = "/boards/{board_key}/dat-archives/{thread_number}/",
        responses(
            (status = 200, description = "Get archived thread successfully", body = ArchivedThread),
        ),
        params(
            ("board_key" = String, Path, description = "Board ID"),
            ("thread_number" = u64, Path, description = "Thread ID"),
        )
    )]
    pub async fn get_dat_archived_thread(
        State(state): State<DefaultAppState>,
        Path((board_key, thread_number)): Path<(String, u64)>,
    ) -> Response {
        match state
            .admin_archive_repo
            .get_thread(&board_key, thread_number)
            .await
        {
            Ok(thread) => Response::builder()
                .status(200)
                .body(serde_json::to_string(&thread).unwrap().into())
                .unwrap(),
            Err(_) => Response::builder()
                .status(500)
                .body(axum::body::Body::empty())
                .unwrap(),
        }
    }

    #[utoipa::path(
        get,
        path = "/boards/{board_key}/admin-dat-archives/{thread_number}/",
        responses(
            (status = 200, description = "Get archived thread successfully", body = ArchivedAdminThread),
        ),
        params(
            ("board_key" = String, Path, description = "Board ID"),
            ("thread_number" = u64, Path, description = "Thread ID"),
        )
    )]
    pub async fn get_admin_dat_archived_thread(
        State(state): State<DefaultAppState>,
        Path((board_key, thread_number)): Path<(String, u64)>,
    ) -> Response {
        match state
            .admin_archive_repo
            .get_archived_admin_thread(&board_key, thread_number)
            .await
        {
            Ok(thread) => Response::builder()
                .status(200)
                .body(serde_json::to_string(&thread).unwrap().into())
                .unwrap(),
            Err(_) => Response::builder()
                .status(500)
                .body(axum::body::Body::empty())
                .unwrap(),
        }
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
        State(state): State<DefaultAppState>,
        Path((_a, _aa, res_id)): Path<(String, u64, Uuid)>,
        Json(body): Json<UpdateResInput>,
    ) -> Response {
        let (res, default_name, board_key, thread_number, thread_title) =
            state.admin_bbs_repo.get_res(res_id).await.unwrap();
        let updated_res = state
            .admin_bbs_repo
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
        patch,
        path = "/boards/{board_key}/dat-archives/{thread_number}/responses/",
        responses(
            (status = 200, description = "Update archived response successfully", body = ()),
        ),
        params(
            ("board_key" = String, Path, description = "Board ID"),
            ("thread_number" = u64, Path, description = "Thread ID"),
        ),
        request_body = Vec<ArchivedResUpdate>,
    )]
    pub async fn update_archived_res(
        State(state): State<DefaultAppState>,
        Path((board_key, thread_number)): Path<(String, u64)>,
        Json(body): Json<Vec<ArchivedResUpdate>>,
    ) -> Response {
        if let Err(e) = state
            .admin_archive_repo
            .update_response(&board_key, thread_number, &body)
            .await
        {
            Response::builder()
                .status(500)
                .body(e.to_string().into())
                .unwrap()
        } else {
            Response::builder()
                .status(200)
                .body(axum::body::Body::empty())
                .unwrap()
        }
    }

    #[utoipa::path(
        delete,
        path = "/boards/{board_key}/dat-archives/{thread_number}/responses/{res_order}/",
        responses(
            (status = 200, description = "Delete response successfully"),
        ),
        params(
            ("board_key" = String, Path, description = "Board ID"),
            ("thread_number" = u64, Path, description = "Thread ID"),
            ("res_order" = u64, Path, description = "Response order"),
        ),
    )]
    pub async fn delete_archived_res(
        State(state): State<DefaultAppState>,
        Path((board_key, thread_number, res_order)): Path<(String, u64, u64)>,
    ) -> Response {
        if let Err(e) = state
            .admin_archive_repo
            .delete_response(&board_key, thread_number, res_order)
            .await
        {
            Response::builder()
                .status(500)
                .body(e.to_string().into())
                .unwrap()
        } else {
            Response::builder()
                .status(200)
                .body(axum::body::Body::empty())
                .unwrap()
        }
    }

    #[utoipa::path(
        delete,
        path = "/boards/{board_key}/dat-archives/{thread_number}/",
        responses(
            (status = 200, description = "Delete thread successfully"),
        ),
        params(
            ("board_key" = String, Path, description = "Board ID"),
            ("thread_number" = u64, Path, description = "Thread ID"),
        ),
    )]
    pub async fn delete_archived_thread(
        State(state): State<DefaultAppState>,
        Path((board_key, thread_number)): Path<(String, u64)>,
    ) -> Response {
        if let Err(e) = state
            .admin_archive_repo
            .delete_thread(&board_key, thread_number)
            .await
        {
            Response::builder()
                .status(500)
                .body(e.to_string().into())
                .unwrap()
        } else {
            Response::builder()
                .status(200)
                .body(axum::body::Body::empty())
                .unwrap()
        }
    }

    #[utoipa::path(
        get,
        path = "/authed_tokens/{authed_token_id}/",
        responses(
            (status = 200, description = "Get authed token successfully", body = AuthedToken),
        ),
        params(
            ("authed_token_id" = Uuid, Path, description = "Authed token ID"),
        ),
    )]
    pub async fn get_authed_token(
        State(state): State<DefaultAppState>,
        Path(authed_token_id): Path<Uuid>,
    ) -> Response {
        let authed_token = state
            .authed_token_repo
            .get_authed_token(authed_token_id)
            .await
            .unwrap();

        Response::builder()
            .status(200)
            .body(serde_json::to_string(&authed_token).unwrap().into())
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
        State(state): State<DefaultAppState>,
        Path(authed_token_id): Path<Uuid>,
        Query(DeleteAuthedTokenInput { using_origin_ip }): Query<DeleteAuthedTokenInput>,
    ) -> Response {
        if using_origin_ip {
            state
                .authed_token_repo
                .delete_authed_token(authed_token_id)
                .await
                .unwrap();
        } else {
            state
                .authed_token_repo
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
    pub async fn get_ng_words(State(state): State<DefaultAppState>) -> Json<Vec<NgWord>> {
        let ng_words = state.ng_word_repo.get_ng_words().await.unwrap();
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
        State(state): State<DefaultAppState>,
        Json(body): Json<CreationNgWordInput>,
    ) -> Response {
        let ng_word = state
            .ng_word_repo
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
        State(state): State<DefaultAppState>,
        Path(ng_word_id): Path<Uuid>,
        Json(body): Json<UpdateNgWordInput>,
    ) -> Response {
        let ng_word = state
            .ng_word_repo
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
        State(state): State<DefaultAppState>,
        Path(ng_word_id): Path<Uuid>,
    ) -> Response {
        state.ng_word_repo.delete_ng_word(ng_word_id).await.unwrap();

        Response::builder()
            .status(200)
            .body(axum::body::Body::empty())
            .unwrap()
    }

    #[utoipa::path(
        get,
        path = "/caps/",
        responses(
            (status = 200, description = "List cap words successfully", body = Vec<Cap>),
        )
    )]
    pub async fn get_caps(State(state): State<DefaultAppState>) -> Json<Vec<Cap>> {
        let caps = state.cap_repo.get_caps().await.unwrap();
        caps.into()
    }

    #[utoipa::path(
        post,
        path = "/caps/",
        responses(
            (status = 200, description = "Create cap successfully", body = Cap),
        ),
        request_body = CreationCapInput
    )]
    pub async fn create_cap(
        State(state): State<DefaultAppState>,
        Json(body): Json<CreationCapInput>,
    ) -> Response {
        let cap = state
            .cap_repo
            .create_cap(
                &body.name,
                &body.description,
                &eddist_core::domain::cap::calculate_cap_hash(
                    &body.password,
                    &std::env::var("TINKER_SECRET").unwrap(),
                ),
            )
            .await
            .unwrap();

        Response::builder()
            .status(200)
            .body(serde_json::to_string(&cap).unwrap().into())
            .unwrap()
    }

    #[utoipa::path(
        patch,
        path = "/caps/{cap_id}/",
        responses(
            (status = 200, description = "Update cap word successfully", body = Cap),
        ),
        params(
            ("cap_id" = Uuid, Path, description = "Cap ID"),
        ),
        request_body = UpdateCapInput
    )]
    pub async fn update_cap(
        State(state): State<DefaultAppState>,
        Path(cap_id): Path<Uuid>,
        Json(body): Json<UpdateCapInput>,
    ) -> Response {
        let cap = state
            .cap_repo
            .update_cap(
                cap_id,
                body.name.as_deref(),
                body.description.as_deref(),
                body.password
                    .map(|x| {
                        eddist_core::domain::cap::calculate_cap_hash(
                            &x,
                            &std::env::var("TINKER_SECRET").unwrap(),
                        )
                    })
                    .as_deref(),
                body.board_ids,
            )
            .await
            .unwrap();

        Response::builder()
            .status(200)
            .body(serde_json::to_string(&cap).unwrap().into())
            .unwrap()
    }

    #[utoipa::path(
        delete,
        path = "/caps/{cap_id}/",
        responses(
            (status = 200, description = "Delete Cap successfully"),
        ),
        params(
            ("cap_id" = Uuid, Path, description = "Cap ID"),
        ),
    )]
    pub async fn delete_cap(
        State(state): State<DefaultAppState>,
        Path(cap_id): Path<Uuid>,
    ) -> Response {
        state.cap_repo.delete_cap(cap_id).await.unwrap();

        Response::builder()
            .status(200)
            .body(axum::body::Body::empty())
            .unwrap()
    }

    #[utoipa::path(
        post,
        path = "/boards/{board_key}/threads-compaction/",
        responses(
            (status = 200, description = "Compaction thread successfully"),
        ),
        params(
            ("board_key" = String, Path, description = "Board Key"),
        ),
        request_body = ThreadCompactionInput
    )]
    pub async fn threads_compaction(
        State(state): State<DefaultAppState>,
        Path(board_key): Path<String>,
        Json(body): Json<ThreadCompactionInput>,
    ) -> Response {
        state
            .admin_bbs_repo
            .compact_threads(&board_key, body.target_count)
            .await
            .unwrap();

        Response::builder()
            .status(200)
            .body(axum::body::Body::empty())
            .unwrap()
    }

    #[derive(Debug, Clone, Serialize, Deserialize, IntoParams)]
    pub struct UserSearchQuery {
        pub user_id: Option<Uuid>,
        pub user_name: Option<String>,
        pub authed_token_id: Option<Uuid>,
    }

    #[utoipa::path(
        get,
        path = "/users/search/",
        params(
            UserSearchQuery
        ),
        responses(
            (status = 200, description = "List users successfully", body = Vec<User>),
        )
    )]
    pub async fn search_users(
        State(state): State<DefaultAppState>,
        Query(query): Query<UserSearchQuery>,
    ) -> Json<Vec<User>> {
        let users = state
            .user_repo
            .search_users(query.user_id, query.user_name, query.authed_token_id)
            .await
            .unwrap();

        Json(users)
    }

    #[utoipa::path(
        patch,
        path = "/users/{user_id}/status/",
        responses(
            (status = 200, description = "Update user status successfully", body = User),
        ),
        params(
            ("user_id" = Uuid, Path, description = "User ID"),
        ),
        request_body = UserStatusUpdateInput
    )]
    pub async fn update_user_status(
        State(state): State<DefaultAppState>,
        Path(user_id): Path<Uuid>,
        Json(body): Json<UserStatusUpdateInput>,
    ) -> Response {
        state
            .user_repo
            .update_user_status(user_id, body.enabled)
            .await
            .unwrap();

        let users = state
            .user_repo
            .search_users(Some(user_id), None, None)
            .await
            .unwrap();

        Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&users[0]).unwrap().into())
            .unwrap()
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(
        bbs::get_boards,
        bbs::get_board,
        bbs::get_board_info,
        bbs::create_board,
        bbs::edit_board,
        bbs::get_threads,
        bbs::get_thread,
        bbs::get_responses,
        bbs::get_archived_threads,
        bbs::get_archived_thread,
        bbs::get_archived_responses,
        bbs::get_dat_archived_thread,
        bbs::get_admin_dat_archived_thread,
        bbs::update_response,
        bbs::update_archived_res,
        bbs::delete_archived_res,
        bbs::delete_archived_thread,
        bbs::get_authed_token,
        bbs::delete_authed_token,
        bbs::get_ng_words,
        bbs::create_ng_word,
        bbs::update_ng_word,
        bbs::delete_ng_word,
        bbs::get_caps,
        bbs::create_cap,
        bbs::update_cap,
        bbs::delete_cap,
        bbs::threads_compaction,
        bbs::search_users,
        bbs::update_user_status,
    ),
    components(schemas(
        Board,
        BoardInfo,
        CreateBoardInput,
        EditBoardInput,
        Thread,
        Res,
        ArchivedThread,
        ArchivedAdminThread,
        ArchivedRes,
        ArchivedAdminRes,
        ArchivedResUpdate,
        ClientInfo,
        Tinker,
        UpdateResInput,
        AuthedToken,
        NgWord,
        CreationNgWordInput,
        UpdateNgWordInput,
        Cap,
        CreationCapInput,
        UpdateCapInput,
        ThreadCompactionInput,
        User,
        UserIdpBinding,
        UserStatusUpdateInput,
    ))
)]
struct ApiDoc;
