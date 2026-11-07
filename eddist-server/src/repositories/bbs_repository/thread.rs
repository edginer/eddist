use chrono::{NaiveDateTime, TimeZone, Utc};
use eddist_core::domain::{
    client_info::ClientInfo,
    ip_addr::{IpAddr, ReducedIpAddr},
};
use eddist_entity::{authed_token, board, db_time, response, thread};
use futures::TryStreamExt;
use sea_orm::{
    ActiveValue::Set,
    ColumnTrait, DbErr, EntityTrait, FromQueryResult, IsolationLevel, JoinType, QueryFilter,
    QueryOrder, QuerySelect, QueryTrait, RelationTrait, Select, SqlErr, TransactionTrait,
    TryGetableFromJson,
    sea_query::{Expr, ExprTrait, Query},
};
use uuid::Uuid;

use crate::domain::{authed_token::AuthedToken, thread::Thread};

use super::{BbsRepositoryImpl, CreatingThread};

#[derive(Debug, Clone, Copy)]
pub enum ThreadStatus {
    // Show in the thread list
    Active,
    // Not show in the thread list and can't be posted (will be archived via eddist-cron)
    Archived,
    // Show in the thread list but can't be posted
    Inactive,
    // Show in the thread list, and it contains the thread that is inactive but not archived
    Unarchived,
}

#[async_trait::async_trait]
pub trait ThreadRepository: Send + Sync + 'static {
    async fn get_threads(
        &self,
        board_id: Uuid,
        status: ThreadStatus,
    ) -> anyhow::Result<Vec<Thread>>;
    async fn get_threads_with_metadent(
        &self,
        board_id: Uuid,
    ) -> anyhow::Result<Vec<(Thread, ClientInfo, AuthedToken)>>;
    async fn get_thread_by_board_key_and_thread_number(
        &self,
        board_key: &str,
        thread_number: u64,
    ) -> anyhow::Result<Option<Thread>>;
    async fn create_thread(&self, thread: CreatingThread) -> Result<(), CreateThreadError>;
}

#[derive(Debug, thiserror::Error)]
pub enum CreateThreadError {
    #[error("thread number already exists")]
    ThreadNumberConflict,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<DbErr> for CreateThreadError {
    fn from(e: DbErr) -> Self {
        Self::Other(e.into())
    }
}

#[async_trait::async_trait]
impl ThreadRepository for BbsRepositoryImpl {
    async fn get_threads(
        &self,
        board_id: Uuid,
        status: ThreadStatus,
    ) -> anyhow::Result<Vec<Thread>> {
        let query = thread_select().filter(thread::Column::BoardId.eq(board_id));
        let query = match status {
            ThreadStatus::Active => query.filter(thread::Column::Active.eq(true)),
            ThreadStatus::Archived => query.filter(thread::Column::Archived.eq(true)),
            ThreadStatus::Inactive => query
                .filter(thread::Column::Active.eq(false))
                .filter(thread::Column::Archived.eq(false)),
            ThreadStatus::Unarchived => query
                .filter(thread::Column::Archived.eq(false))
                .order_by_desc(thread::Column::SageLastModifiedAt),
        };
        let threads = query.into_model::<SelectionThread>().all(&self.db).await?;

        Ok(threads
            .into_iter()
            .map(SelectionThread::into_thread)
            .collect())
    }

    async fn get_threads_with_metadent(
        &self,
        board_id: Uuid,
    ) -> anyhow::Result<Vec<(Thread, ClientInfo, AuthedToken)>> {
        // Decode typed rows as they arrive on large thread lists.
        let threads = metadent_select(board_id)
            .into_model::<SelectionThreadWithMetadent>()
            .stream(&self.db)
            .await?
            .try_collect::<Vec<_>>()
            .await?;

        Ok(threads
            .into_iter()
            .map(|x| {
                (
                    Thread {
                        id: x.id,
                        board_id: x.board_id,
                        thread_number: x.thread_number,
                        last_modified_at: Utc.from_utc_datetime(&x.last_modified_at),
                        sage_last_modified_at: Utc.from_utc_datetime(&x.sage_last_modified_at),
                        title: x.title,
                        authed_token_id: x.authed_token_id,
                        metadent: x.metadent,
                        response_count: x.response_count as u32,
                        no_pool: x.no_pool,
                        active: x.active,
                        archived: x.archived,
                    },
                    x.client_info.0,
                    AuthedToken {
                        id: x.authed_token_id,
                        token: x.token,
                        origin_ip: IpAddr::new(x.origin_ip),
                        reduced_ip: ReducedIpAddr::from(x.reduced_origin_ip),
                        asn_num: 0,
                        writing_ua: x.writing_ua,
                        authed_ua: x.authed_ua,
                        auth_code: x.auth_code,
                        created_at: Utc.from_utc_datetime(&x.created_at),
                        authed_at: x.authed_at.map(|dt| Utc.from_utc_datetime(&dt)),
                        validity: x.validity,
                        last_wrote_at: x.last_wrote_at.map(|dt| Utc.from_utc_datetime(&dt)),
                        author_id_seed: x.author_id_seed,
                        require_user_registration: x.require_user_registration,
                        registered_user_id: x.registered_user_id,
                        require_reauth: x.require_reauth,
                    },
                )
            })
            .collect())
    }

    async fn get_thread_by_board_key_and_thread_number(
        &self,
        board_key: &str,
        thread_number: u64,
    ) -> anyhow::Result<Option<Thread>> {
        let board = board::Entity::find()
            .select_only()
            .column(board::Column::Id)
            .filter(board::Column::BoardKey.eq(board_key))
            .limit(1)
            .into_query();
        let th = thread_select()
            .filter(thread::Column::ThreadNumber.eq(thread_number))
            .filter(Expr::col((thread::Entity, thread::Column::BoardId)).eq(Expr::from(board)))
            .into_model::<SelectionThread>()
            .one(&self.db)
            .await?;

        Ok(th.map(SelectionThread::into_thread))
    }

    async fn create_thread(&self, thread: CreatingThread) -> Result<(), CreateThreadError> {
        let created_at = db_time::truncate_to_millis(thread.created_at.naive_utc());
        let model = thread::ActiveModel {
            id: Set(thread.thread_id),
            board_id: Set(thread.board_id),
            thread_number: Set(thread.unix_time as i64),
            last_modified_at: Set(created_at),
            sage_last_modified_at: Set(created_at),
            title: Set(thread.title),
            authed_token_id: Set(thread.authed_token_id),
            metadent: Set(Option::<&str>::from(thread.metadent).unwrap_or("").into()),
            response_count: Set(1),
            ..Default::default()
        };
        let first_response =
            super::response::response_model(eddist_core::domain::pubsub_repository::CreatingRes {
                id: thread.response_id,
                created_at: thread.created_at,
                body: thread.body,
                name: thread.name,
                mail: thread.mail,
                author_ch5id: thread.author_ch5id,
                authed_token_id: thread.authed_token_id,
                ip_addr: thread.ip_addr,
                thread_id: thread.thread_id,
                board_id: thread.board_id,
                client_info: thread.client_info,
                res_order: 1,
                is_sage: false,
                moderation_result: thread.moderation_result,
            })?;

        let tx = self
            .db
            .begin_with_config(Some(IsolationLevel::ReadCommitted), None)
            .await?;
        thread::Entity::insert(model)
            .exec_without_returning(&tx)
            .await
            .map_err(|e| match e.sql_err() {
                Some(SqlErr::UniqueConstraintViolation(_)) => {
                    CreateThreadError::ThreadNumberConflict
                }
                _ => e.into(),
            })?;
        eddist_entity::response::Entity::insert(first_response)
            .exec_without_returning(&tx)
            .await?;
        tx.commit().await?;

        Ok(())
    }
}

fn thread_select() -> Select<thread::Entity> {
    use thread::Column;
    thread::Entity::find().select_only().columns([
        Column::Id,
        Column::BoardId,
        Column::ThreadNumber,
        Column::LastModifiedAt,
        Column::SageLastModifiedAt,
        Column::Title,
        Column::AuthedTokenId,
        Column::Metadent,
        Column::ResponseCount,
        Column::NoPool,
        Column::Active,
        Column::Archived,
        Column::ArchiveConverted,
    ])
}

fn metadent_select(board_id: Uuid) -> Select<thread::Entity> {
    // This scalar subquery must not gain LIMIT / DISTINCT or become an INNER
    // JOIN: missing and duplicate first responses must still fail decoding/SQL.
    let client_info = Query::select()
        .column((response::Entity, response::Column::ClientInfo))
        .from(response::Entity)
        .and_where(
            Expr::col((response::Entity, response::Column::ThreadId))
                .equals((thread::Entity, thread::Column::Id)),
        )
        .and_where(Expr::col((response::Entity, response::Column::ResOrder)).eq(1))
        .to_owned();
    use authed_token::Column;
    thread_select()
        .column_as(Expr::from(client_info), "client_info")
        .columns([
            Column::Token,
            Column::OriginIp,
            Column::ReducedOriginIp,
            Column::WritingUa,
            Column::AuthedUa,
            Column::AuthCode,
            Column::CreatedAt,
            Column::AuthedAt,
            Column::Validity,
            Column::LastWroteAt,
            Column::AuthorIdSeed,
            Column::RequireUserRegistration,
            Column::RegisteredUserId,
            Column::RequireReauth,
        ])
        .join(JoinType::InnerJoin, thread::Relation::AuthedToken.def())
        .filter(thread::Column::BoardId.eq(board_id))
        .filter(thread::Column::Archived.eq(false))
        .order_by_desc(thread::Column::SageLastModifiedAt)
}

#[derive(Debug, FromQueryResult)]
struct SelectionThread {
    id: Uuid,
    board_id: Uuid,
    thread_number: i64,
    last_modified_at: NaiveDateTime,
    sage_last_modified_at: NaiveDateTime,
    title: String,
    authed_token_id: Uuid,
    metadent: String,
    response_count: i32,
    no_pool: bool,
    active: bool,
    archived: bool,
    archive_converted: bool,
}

impl SelectionThread {
    fn into_thread(self) -> Thread {
        Thread {
            id: self.id,
            board_id: self.board_id,
            thread_number: self.thread_number,
            last_modified_at: Utc.from_utc_datetime(&self.last_modified_at),
            sage_last_modified_at: Utc.from_utc_datetime(&self.sage_last_modified_at),
            title: self.title,
            authed_token_id: self.authed_token_id,
            metadent: self.metadent,
            response_count: self.response_count as u32,
            no_pool: self.no_pool,
            active: self.active,
            archived: self.archived,
        }
    }
}

// Decode the typed JSON directly, retaining SQL NULL and malformed JSON errors
// without an intermediate serde_json::Value tree.
#[derive(Debug, serde::Deserialize)]
#[serde(transparent)]
struct ClientInfoJson(ClientInfo);

impl TryGetableFromJson for ClientInfoJson {}

#[derive(Debug, FromQueryResult)]
struct SelectionThreadWithMetadent {
    id: Uuid,
    board_id: Uuid,
    thread_number: i64,
    last_modified_at: NaiveDateTime,
    sage_last_modified_at: NaiveDateTime,
    title: String,
    authed_token_id: Uuid,
    metadent: String,
    response_count: i32,
    no_pool: bool,
    active: bool,
    archived: bool,
    archive_converted: bool,
    client_info: ClientInfoJson,

    token: String,
    origin_ip: String,
    reduced_origin_ip: String,
    writing_ua: String,
    authed_ua: Option<String>,
    auth_code: String,
    created_at: NaiveDateTime,
    authed_at: Option<NaiveDateTime>,
    validity: bool,
    last_wrote_at: Option<NaiveDateTime>,
    author_id_seed: Vec<u8>,
    require_user_registration: bool,
    registered_user_id: Option<Uuid>,
    require_reauth: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::DbBackend;

    #[test]
    fn metadent_keeps_an_unlimited_scalar_subquery_in_both_dialects() {
        for backend in [DbBackend::MySql, DbBackend::Postgres] {
            let sql = metadent_select(Uuid::nil()).build(backend).to_string();
            let sql = sql.replace(['`', '"'], "");
            assert!(sql.contains("(SELECT responses.client_info FROM responses WHERE responses.thread_id = threads.id AND responses.res_order = 1) AS client_info"), "{sql}");
            assert!(
                sql.contains(
                    "INNER JOIN authed_tokens ON threads.authed_token_id = authed_tokens.id"
                ),
                "{sql}"
            );
            assert!(
                sql.contains(
                    "AND threads.archived = FALSE ORDER BY threads.sage_last_modified_at DESC"
                ),
                "{sql}"
            );
            for forbidden in [
                "JOIN responses",
                "LIMIT",
                "DISTINCT",
                "additional_info",
                "asn_num",
            ] {
                assert!(!sql.contains(forbidden), "unexpected {forbidden}: {sql}");
            }
        }
    }
}
