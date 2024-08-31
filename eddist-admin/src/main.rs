use async_graphql::{http::GraphiQLSource, EmptySubscription, Schema};
use async_graphql_axum::GraphQL;
use auth::{auth_simple_header, get_check_auth, get_login, get_login_callback, get_logout};
use axum::{
    body::Body,
    extract::{MatchedPath, Request},
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::{Html, IntoResponse, Response},
    routing::{get, post_service},
    Router,
};
use eddist_core::utils::is_prod;
use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};
use repository::admin_bbs_repository::{AdminBbsRepository, AdminBbsRepositoryImpl};
use time::Duration;
use tokio::net::TcpListener;
use tower_sessions::{cookie::SameSite, Expiry, MemoryStore, SessionManagerLayer};
use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};

use std::net::SocketAddr;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing::info_span;

pub(crate) mod auth;
pub(crate) mod graphql;
pub(crate) mod repository {
    pub mod admin_bbs_repository;
    pub mod admin_repository;
}
pub(crate) mod role;

async fn graphiql() -> impl IntoResponse {
    Html(GraphiQLSource::build().endpoint("/api/graphql").finish())
}

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

#[tokio::main]
async fn main() {
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
        "eddist-admin/client/dist"
    };
    let serve_dir = ServeDir::new(serve_dir)
        .not_found_service(ServeFile::new(format!("{serve_dir}/index.html")));

    let pool = sqlx::mysql::MySqlPool::connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();

    let schema = Schema::build(graphql::Query, graphql::Mutation, EmptySubscription)
        .data::<Box<dyn AdminBbsRepository>>(Box::new(AdminBbsRepositoryImpl::new(pool.clone())))
        .finish();

    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(Duration::days(14)));

    let state = (client, pool);

    let app = Router::new()
        .route("/login", get(get_login))
        .route("/auth/check", get(get_check_auth))
        .route("/auth/logout", get(get_logout))
        .route("/auth/callback", get(get_login_callback))
        .route("/api/graphiql", get(graphiql))
        .route(
            "/api/graphql",
            post_service(GraphQL::new(schema))
                .layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    auth_simple_header,
                ))
                .options(ok),
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
        .with_state(state);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
