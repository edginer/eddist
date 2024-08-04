use async_graphql::{
    http::GraphiQLSource, Context, EmptySubscription, FieldResult, InputObject, Object, Schema, ID,
};
use async_graphql_axum::GraphQL;
use axum::{
    body::Body,
    extract::{MatchedPath, Request},
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::{Html, IntoResponse, Response},
    routing::{get, post_service},
    Router,
};
use chrono::Utc;
use repository::{AdminBbsRepository, AdminBbsRepositoryImpl};
use tokio::net::TcpListener;
use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};

use std::net::SocketAddr;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing::info_span;

pub(crate) mod repository;

struct Query;

#[Object]
impl Query {
    async fn hello(&self) -> FieldResult<String> {
        Ok("Hello, world!".to_string())
    }

    async fn boards(&self, ctx: &Context<'_>, board_keys: Vec<String>) -> FieldResult<Vec<Board>> {
        let repo = ctx.data::<Box<dyn AdminBbsRepository>>().unwrap();
        let boards = repo
            .get_boards_by_key(if board_keys.is_empty() {
                None
            } else {
                Some(board_keys)
            })
            .await?;

        Ok(boards)
    }

    async fn audit_logs(&self, ctx: &Context<'_>) -> FieldResult<Vec<AuditLog>> {
        // Implement your logic here
        Ok(vec![]) // Replace with actual data retrieval
    }

    async fn ng_words(&self, ctx: &Context<'_>) -> FieldResult<Vec<NgWord>> {
        // Implement your logic here
        Ok(vec![]) // Replace with actual data retrieval
    }
}

#[derive(Debug)]
pub struct Board {
    pub id: ID,
    pub name: String,
    pub board_key: String,
    pub local_rule: String,
    pub default_name: String,
    pub thread_count: i64,
}

#[Object]
impl Board {
    async fn id(&self) -> &ID {
        &self.id
    }

    async fn name(&self) -> &String {
        &self.name
    }

    async fn board_key(&self) -> &String {
        &self.board_key
    }

    async fn local_rule(&self) -> &String {
        &self.local_rule
    }

    async fn default_name(&self) -> &String {
        &self.default_name
    }

    async fn thread_count(&self) -> i64 {
        self.thread_count
    }

    async fn threads(
        &self,
        ctx: &Context<'_>,
        thread_number: Vec<u64>,
    ) -> FieldResult<Vec<Thread>> {
        let repo = ctx.data::<Box<dyn AdminBbsRepository>>().unwrap();

        let threads = repo
            .get_threads_by_thread_id(
                &self.board_key,
                if thread_number.is_empty() {
                    None
                } else {
                    Some(thread_number)
                },
            )
            .await?;
        Ok(threads)
    }
}

pub struct Thread {
    pub id: ID,
    pub board_id: ID,
    pub thread_number: u64,
    pub last_modified: chrono::DateTime<Utc>,
    pub sage_last_modified: chrono::DateTime<Utc>,
    pub title: String,
    pub authed_token_id: ID,
    pub metadent: String,
    pub response_count: u32,
    pub no_pool: bool,
    pub archived: bool,
    pub active: bool,
}

#[Object]
impl Thread {
    async fn id(&self) -> &ID {
        &self.id
    }

    async fn thread_number(&self) -> u64 {
        self.thread_number
    }

    async fn title(&self) -> &String {
        &self.title
    }

    async fn response_count(&self) -> u32 {
        self.response_count
    }

    async fn last_modified(&self) -> chrono::DateTime<Utc> {
        self.last_modified
    }

    async fn board_id(&self) -> &ID {
        &self.board_id
    }

    async fn archived(&self) -> bool {
        self.archived
    }

    async fn active(&self) -> bool {
        self.active
    }

    async fn no_pool(&self) -> bool {
        self.no_pool
    }

    async fn metadent(&self) -> &str {
        &self.metadent
    }

    async fn authed_token_id(&self) -> &ID {
        &self.authed_token_id
    }

    async fn sage_last_modified(&self) -> chrono::DateTime<Utc> {
        self.sage_last_modified
    }

    async fn responses(&self, ctx: &Context<'_>) -> FieldResult<Vec<Res>> {
        let repo = ctx.data::<Box<dyn AdminBbsRepository>>().unwrap();
        let reses = repo
            .get_reses_by_thread_id(self.board_id.0.parse()?, self.id.0.parse()?)
            .await?;
        Ok(reses) // Replace with actual data retrieval
    }
}

pub struct Res {
    pub id: ID,
    pub author_name: Option<String>,
    pub mail: Option<String>,
    pub body: String,
    pub created_at: chrono::DateTime<Utc>,
    pub author_id: String,
    pub ip_addr: String,
    pub authed_token_id: ID,
    pub board_id: ID,
    pub thread_id: ID,
    pub is_abone: bool,
    pub res_order: i32,
}

#[Object]
impl Res {
    async fn id(&self) -> &ID {
        &self.id
    }

    async fn author_name(&self) -> Option<&String> {
        self.author_name.as_ref()
    }

    async fn mail(&self) -> Option<&String> {
        self.mail.as_ref()
    }

    async fn body(&self) -> &str {
        &self.body
    }

    async fn created_at(&self) -> &chrono::DateTime<Utc> {
        &self.created_at
    }

    async fn author_id(&self) -> &str {
        &self.author_id
    }

    async fn ip_addr(&self) -> &str {
        &self.ip_addr
    }

    async fn authed_token_id(&self) -> &ID {
        &self.authed_token_id
    }

    async fn board_id(&self) -> &ID {
        &self.board_id
    }

    async fn thread_id(&self) -> &ID {
        &self.thread_id
    }

    async fn is_abone(&self) -> bool {
        self.is_abone
    }

    async fn res_order(&self) -> i32 {
        self.res_order
    }
}

pub struct AuditLog {
    pub id: i32,
    pub user_email: String,
    pub used_permission: String,
    pub info: String,
    pub ip_addr: String,
    pub timestamp: String,
}

#[Object]
impl AuditLog {
    async fn id(&self) -> i32 {
        self.id
    }

    async fn user_email(&self) -> &String {
        &self.user_email
    }

    async fn used_permission(&self) -> &String {
        &self.used_permission
    }

    async fn info(&self) -> &String {
        &self.info
    }

    async fn ip_addr(&self) -> &String {
        &self.ip_addr
    }

    async fn timestamp(&self) -> &String {
        &self.timestamp
    }
}

pub struct NgWord {
    pub id: i32,
    pub name: String,
    pub value: String,
    pub restriction_type: String,
}

#[Object]
impl NgWord {
    async fn id(&self) -> i32 {
        self.id
    }

    async fn name(&self) -> &String {
        &self.name
    }

    async fn value(&self) -> &String {
        &self.value
    }

    async fn restriction_type(&self) -> &String {
        &self.restriction_type
    }
}

pub struct Mutation;

#[Object]
impl Mutation {
    async fn update_response(&self, ctx: &Context<'_>, res: ResInput) -> FieldResult<Res> {
        let repo = ctx.data::<Box<dyn AdminBbsRepository>>().unwrap();

        Ok(repo
            .update_res(
                res.id.0.parse()?,
                res.author_name,
                res.mail,
                res.body,
                res.is_abone,
            )
            .await?)
    }

    async fn delete_authed_token(
        &self,
        ctx: &Context<'_>,
        input: DeleteAuthedTokenInput,
    ) -> FieldResult<bool> {
        let repo = ctx.data::<Box<dyn AdminBbsRepository>>().unwrap();

        if input.using_origin_ip {
            repo.delete_authed_token(input.token_id.parse()?).await?;
        } else {
            repo.delete_authed_token_by_origin_ip(input.token_id.parse()?)
                .await?;
        }
        Ok(true)
    }

    async fn update_ng_word(&self, ctx: &Context<'_>, ng_word: NgWordInput) -> FieldResult<NgWord> {
        todo!()
    }

    async fn add_ng_word(&self, ctx: &Context<'_>, ng_word: NgWordAddInput) -> FieldResult<NgWord> {
        todo!()
    }

    async fn delete_ng_word(&self, ctx: &Context<'_>, id: i32) -> FieldResult<bool> {
        todo!()
    }
}

#[derive(InputObject)]
struct ResInput {
    id: ID,
    author_name: Option<String>,
    mail: Option<String>,
    body: Option<String>,
    is_abone: Option<bool>,
}

#[derive(InputObject)]
struct NgWordInput {
    id: i32,
    name: String,
    value: String,
    restriction_type: String,
}

#[derive(InputObject)]
struct NgWordAddInput {
    name: String,
    value: String,
    restriction_type: String,
}

#[derive(InputObject)]
struct DeleteAuthedTokenInput {
    token_id: String,
    using_origin_ip: bool,
}

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

async fn auth_simple_header(req: Request<Body>, next: Next) -> Response {
    if !matches!(
        std::env::var("RUST_ENV").as_deref(),
        Ok("prod" | "production")
    ) {
        return next.run(req).await;
    }

    if let Some(authorization) = req.headers().get("Authorization") {
        let env_simple_auth = std::env::var("SIMPLE_AUTH").unwrap();
        if authorization.to_str().unwrap() == env_simple_auth {
            return next.run(req).await;
        }
    }

    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .body(Body::empty())
        .unwrap()
}

async fn ok() -> impl IntoResponse {
    StatusCode::OK
}

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([0, 0, 0, 0], 8081));
    if !matches!(
        std::env::var("RUST_ENV").as_deref(),
        Ok("prod" | "production")
    ) {
        dotenvy::dotenv().unwrap();
    }

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_span_events(FmtSpan::CLOSE)
        .with_ansi(false)
        .init();

    let serve_dir = ServeDir::new("dist").not_found_service(ServeFile::new("dist/index.html"));

    let pool = sqlx::mysql::MySqlPool::connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .unwrap();

    let schema = Schema::build(Query, Mutation, EmptySubscription)
        .data::<Box<dyn AdminBbsRepository>>(Box::new(AdminBbsRepositoryImpl::new(pool)))
        .finish();

    let app = Router::new()
        .route("/api/graphiql", get(graphiql))
        .route(
            "/api/graphql",
            post_service(GraphQL::new(schema))
                .layer(axum::middleware::from_fn(add_cors_header))
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
        .layer(axum::middleware::from_fn(add_cors_header));

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
