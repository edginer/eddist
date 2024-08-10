use async_graphql::{Context, FieldResult, InputObject, Object, ID};
use chrono::Utc;
use eddist_core::domain::client_info::ClientInfo;

use crate::repository::admin_bbs_repository::AdminBbsRepository;

pub struct Query;

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
    pub client_info: ClientInfo,
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

    async fn client_info(&self) -> GqlClientInfo {
        GqlClientInfo {
            user_agent: self.client_info.user_agent.clone(),
            asn_num: self.client_info.asn_num as i32,
            ip_addr: self.client_info.ip_addr.clone(),
            tinker: self.client_info.tinker.as_ref().map(|x| GqlTinker {
                authed_token: x.authed_token().to_string(),
                wrote_count: x.wrote_count(),
                created_thread_count: x.created_thread_count(),
                level: x.level(),
                last_level_up_at: x.last_level_up_at(),
                last_wrote_at: x.last_wrote_at(),
            }),
        }
    }

    async fn res_order(&self) -> i32 {
        self.res_order
    }
}

pub struct GqlClientInfo {
    pub user_agent: String,
    pub asn_num: i32,
    pub ip_addr: String,
    pub tinker: Option<GqlTinker>,
}

#[Object]
impl GqlClientInfo {
    async fn user_agent(&self) -> &String {
        &self.user_agent
    }

    async fn asn_num(&self) -> i32 {
        self.asn_num
    }

    async fn ip_addr(&self) -> &String {
        &self.ip_addr
    }

    async fn tinker(&self) -> Option<&GqlTinker> {
        self.tinker.as_ref()
    }
}

pub struct GqlTinker {
    authed_token: String,
    wrote_count: u32,
    created_thread_count: u32,
    level: u32,
    last_level_up_at: u64,
    last_wrote_at: u64,
}

#[Object]
impl GqlTinker {
    async fn authed_token(&self) -> &String {
        &self.authed_token
    }

    async fn wrote_count(&self) -> u32 {
        self.wrote_count
    }

    async fn created_thread_count(&self) -> u32 {
        self.created_thread_count
    }

    async fn level(&self) -> u32 {
        self.level
    }

    async fn last_level_up_at(&self) -> u64 {
        self.last_level_up_at
    }

    async fn last_wrote_at(&self) -> u64 {
        self.last_wrote_at
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
