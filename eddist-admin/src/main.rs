use async_graphql::{
    http::GraphiQLSource, Context, EmptyMutation, EmptySubscription, FieldResult, InputObject,
    Object, Schema, ID,
};
use async_graphql_axum::GraphQL;
use axum::{
    extract::{MatchedPath, Request},
    response::{Html, IntoResponse},
    routing::{get, post_service},
    Router,
};
use tokio::net::TcpListener;

use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use tracing::info_span;

struct Query;

#[Object]
impl Query {
    async fn hello(&self) -> FieldResult<String> {
        Ok("Hello, world!".to_string())
    }

    async fn boards(&self, ctx: &Context<'_>) -> FieldResult<Vec<Board>> {
        // Implement your logic here
        Ok(vec![]) // Replace with actual data retrieval
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

pub struct Board {
    pub id: ID,
    pub name: String,
    pub board_key: String,
    pub local_rule: String,
    pub default_name: String,
    pub thread_count: i32,
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

    async fn thread_count(&self) -> i32 {
        self.thread_count
    }

    async fn threads(
        &self,
        ctx: &Context<'_>,
        thread_number: Option<String>,
    ) -> FieldResult<Vec<Thread>> {
        // Implement your logic here
        Ok(vec![]) // Replace with actual data retrieval
    }

    async fn archived_threads(
        &self,
        ctx: &Context<'_>,
        page: Option<i32>,
        query: Option<String>,
        thread_id: Option<String>,
    ) -> FieldResult<Vec<ArchivedThread>> {
        // Implement your logic here
        Ok(vec![]) // Replace with actual data retrieval
    }
}

pub struct Thread {
    pub thread_number: String,
    pub title: String,
    pub response_count: i32,
    pub last_modified: String,
    pub board_id: i32,
    pub non_auth_thread: i32,
    pub archived: i32,
    pub active: i32,
    pub authed_cookie: Option<String>,
    pub modulo: i32,
}

#[Object]
impl Thread {
    async fn thread_number(&self) -> &String {
        &self.thread_number
    }

    async fn title(&self) -> &String {
        &self.title
    }

    async fn response_count(&self) -> i32 {
        self.response_count
    }

    async fn last_modified(&self) -> &String {
        &self.last_modified
    }

    async fn board_id(&self) -> i32 {
        self.board_id
    }

    async fn non_auth_thread(&self) -> i32 {
        self.non_auth_thread
    }

    async fn archived(&self) -> i32 {
        self.archived
    }

    async fn active(&self) -> i32 {
        self.active
    }

    async fn authed_cookie(&self) -> &Option<String> {
        &self.authed_cookie
    }

    async fn responses(&self, ctx: &Context<'_>, id: Option<i32>) -> FieldResult<Vec<Res>> {
        // Implement your logic here
        Ok(vec![]) // Replace with actual data retrieval
    }

    async fn modulo(&self) -> i32 {
        self.modulo
    }
}

pub struct Res {
    pub id: ID,
    pub name: Option<String>,
    pub mail: Option<String>,
    pub date: String,
    pub author_id: Option<String>,
    pub body: String,
    pub thread_id: String,
    pub ip_addr: String,
    pub authed_token: Option<String>,
    pub timestamp: i32,
    pub board_id: i32,
    pub is_abone: bool,
}

#[Object]
impl Res {
    async fn id(&self) -> &ID {
        &self.id
    }

    async fn name(&self) -> &Option<String> {
        &self.name
    }

    async fn mail(&self) -> &Option<String> {
        &self.mail
    }

    async fn date(&self) -> &String {
        &self.date
    }

    async fn author_id(&self) -> &Option<String> {
        &self.author_id
    }

    async fn body(&self) -> &String {
        &self.body
    }

    async fn thread_id(&self) -> &String {
        &self.thread_id
    }

    async fn ip_addr(&self) -> &String {
        &self.ip_addr
    }

    async fn authed_token(&self) -> &Option<String> {
        &self.authed_token
    }

    async fn timestamp(&self) -> i32 {
        self.timestamp
    }

    async fn board_id(&self) -> i32 {
        self.board_id
    }

    async fn is_abone(&self) -> bool {
        self.is_abone
    }
}

pub struct ArchivedThread {
    pub thread_number: String,
    pub title: String,
    pub response_count: i32,
    pub board_id: i32,
    pub last_modified: String,
}

#[Object]
impl ArchivedThread {
    async fn thread_number(&self) -> &String {
        &self.thread_number
    }

    async fn title(&self) -> &String {
        &self.title
    }

    async fn response_count(&self) -> i32 {
        self.response_count
    }

    async fn board_id(&self) -> i32 {
        self.board_id
    }

    async fn last_modified(&self) -> &String {
        &self.last_modified
    }

    async fn responses(&self, ctx: &Context<'_>) -> FieldResult<Vec<ArchivedRes>> {
        // Implement your logic here
        Ok(vec![]) // Replace with actual data retrieval
    }
}

pub struct ArchivedRes {
    pub name: Option<String>,
    pub mail: Option<String>,
    pub date: String,
    pub author_id: Option<String>,
    pub body: String,
    pub ip_addr: String,
    pub authed_token: Option<String>,
    pub is_abone: bool,
}

#[Object]
impl ArchivedRes {
    async fn name(&self) -> &Option<String> {
        &self.name
    }

    async fn mail(&self) -> &Option<String> {
        &self.mail
    }

    async fn date(&self) -> &String {
        &self.date
    }

    async fn author_id(&self) -> &Option<String> {
        &self.author_id
    }

    async fn body(&self) -> &String {
        &self.body
    }

    async fn ip_addr(&self) -> &String {
        &self.ip_addr
    }

    async fn authed_token(&self) -> &Option<String> {
        &self.authed_token
    }

    async fn is_abone(&self) -> bool {
        self.is_abone
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
        // Implement your logic here
        Ok(Res {
            id: res.id,
            name: res.name,
            mail: res.mail,
            date: "2023-01-01T00:00:00Z".to_string(),
            author_id: None,
            body: res.body,
            thread_id: res.thread_id,
            ip_addr: "127.0.0.1".to_string(),
            authed_token: None,
            timestamp: 0,
            board_id: res.board_id,
            is_abone: res.is_abone.unwrap_or(false),
        })
    }

    async fn delete_authed_token(
        &self,
        ctx: &Context<'_>,
        token: String,
        using_origin_ip: bool,
    ) -> FieldResult<bool> {
        // Implement your logic here
        Ok(true)
    }

    async fn update_ng_word(&self, ctx: &Context<'_>, ng_word: NgWordInput) -> FieldResult<NgWord> {
        // Implement your logic here
        Ok(NgWord {
            id: ng_word.id,
            name: ng_word.name,
            value: ng_word.value,
            restriction_type: ng_word.restriction_type,
        })
    }

    async fn add_ng_word(&self, ctx: &Context<'_>, ng_word: NgWordAddInput) -> FieldResult<NgWord> {
        // Implement your logic here
        Ok(NgWord {
            id: 1, // Replace with actual ID assignment
            name: ng_word.name,
            value: ng_word.value,
            restriction_type: ng_word.restriction_type,
        })
    }

    async fn delete_ng_word(&self, ctx: &Context<'_>, id: i32) -> FieldResult<bool> {
        // Implement your logic here
        Ok(true)
    }
}

#[derive(InputObject)]
struct ResInput {
    id: ID,
    name: Option<String>,
    mail: Option<String>,
    body: String,
    thread_id: String,
    board_id: i32,
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

async fn graphiql() -> impl IntoResponse {
    Html(GraphiQLSource::build().endpoint("/api/graphql").finish())
}

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([0, 0, 0, 0], 8081));

    let schema = Schema::build(Query, Mutation, EmptySubscription).finish();

    let app = Router::new()
        .route("/api/graphiql", get(graphiql))
        .route("/api/graphql", post_service(GraphQL::new(schema)))
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
        );

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
