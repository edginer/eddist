use auth::{
    auth_simple_header, get_check_auth, get_login, get_login_callback, get_logout,
    post_native_session,
};
use axum::{
    body::Body,
    extract::{MatchedPath, Request},
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router, ServiceExt,
};
use eddist_core::{tracing::init_tracing, utils::is_prod};
use oauth2::{AuthUrl, ClientId, ClientSecret, EndpointNotSet, EndpointSet, RedirectUrl, TokenUrl};
use repository::{
    admin_archive_repository::AdminArchiveRepositoryImpl,
    admin_bbs_repository::AdminBbsRepositoryImpl, admin_user_repository::AdminUserRepositoryImpl,
    authed_token_repository::AuthedTokenRepositoryImpl, cap_repository::CapRepositoryImpl,
    ngword_repository::NgWordRepositoryImpl, notice_repository::NoticeRepositoryImpl,
    user_restriction_repository::UserRestrictionRepositoryImpl,
};
use s3::creds::Credentials;
use time::Duration;
use tokio::net::TcpListener;
use tower_layer::Layer;
use tower_sessions::{cookie::SameSite, Expiry, MemoryStore, SessionManagerLayer};

use std::{env, net::SocketAddr};
use tower_http::{
    normalize_path::NormalizePathLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing::info_span;

mod api_doc;
mod auth;
mod models;
mod repository {
    pub mod admin_archive_repository;
    pub mod admin_bbs_repository;
    pub mod admin_repository;
    pub mod admin_user_repository;
    pub mod authed_token_repository;
    pub mod cap_repository;
    pub mod ngword_repository;
    pub mod notice_repository;
    pub mod user_restriction_repository;
}
mod role;
mod routes;

use api_doc::ApiDoc;
use repository::{
    admin_archive_repository::AdminArchiveRepository, admin_bbs_repository::AdminBbsRepository,
    admin_user_repository::AdminUserRepository, authed_token_repository::AuthedTokenRepository,
    cap_repository::CapRepository, ngword_repository::NgWordRepository,
    notice_repository::NoticeRepository, user_restriction_repository::UserRestrictionRepository,
};
use utoipa::OpenApi;

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
pub(crate) struct AppState<
    T: AdminBbsRepository + Clone,
    R: AdminArchiveRepository + Clone,
    N: NgWordRepository + Clone,
    A: AuthedTokenRepository + Clone,
    C: CapRepository + Clone,
    U: AdminUserRepository + Clone,
    UR: UserRestrictionRepository + Clone,
    NO: NoticeRepository + Clone,
> {
    pub oauth2_client: oauth2::basic::BasicClient<
        EndpointSet,
        EndpointNotSet,
        EndpointNotSet,
        EndpointNotSet,
        EndpointSet,
    >,
    pub admin_bbs_repo: T,
    pub ng_word_repo: N,
    pub admin_archive_repo: R,
    pub authed_token_repo: A,
    pub cap_repo: C,
    pub user_repo: U,
    pub user_restriction_repo: UR,
    pub notice_repo: NO,
    pub redis_conn: redis::aio::ConnectionManager,
}

pub(crate) type DefaultAppState = AppState<
    AdminBbsRepositoryImpl,
    AdminArchiveRepositoryImpl,
    NgWordRepositoryImpl,
    AuthedTokenRepositoryImpl,
    CapRepositoryImpl,
    AdminUserRepositoryImpl,
    UserRestrictionRepositoryImpl,
    NoticeRepositoryImpl,
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

    let api_routes = routes::create_api_routes();

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
        user_repo: AdminUserRepositoryImpl::new(pool.clone()),
        user_restriction_repo: UserRestrictionRepositoryImpl::new(pool.clone()),
        notice_repo: NoticeRepositoryImpl::new(pool),
    };

    let app = Router::new()
        .route("/health-check", get(ok))
        .route("/login", get(get_login))
        .route("/auth/check", get(get_check_auth))
        .route("/auth/logout", get(get_logout))
        .route("/auth/callback", get(get_login_callback))
        .route("/auth/native/session", post(post_native_session))
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
