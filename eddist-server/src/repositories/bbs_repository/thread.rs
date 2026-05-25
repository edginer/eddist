use chrono::{NaiveDateTime, TimeZone, Utc};
use eddist_core::domain::{
    client_info::ClientInfo,
    ip_addr::{IpAddr, ReducedIpAddr},
};
use sqlx::{query, query_as, types::Json};
use uuid::Uuid;

use crate::domain::{authed_token::AuthedToken, thread::Thread};

use super::{BbsRepositoryImpl, CreatingThread};

#[derive(Debug, Clone, Copy)]
pub enum ThreadStatus {
    // Show in the thread list
    Active,
    // Not show in the thread list and can't be posted (will be archived via eddiner-archiver)
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
    async fn create_thread(&self, thread: CreatingThread) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
impl ThreadRepository for BbsRepositoryImpl {
    async fn get_threads(
        &self,
        board_id: Uuid,
        status: ThreadStatus,
    ) -> anyhow::Result<Vec<Thread>> {
        let board_id = Vec::<u8>::from(board_id);

        let threads = match status {
            ThreadStatus::Active => {
                query_as!(
                    SelectionThread,
                    r"SELECT * FROM threads WHERE board_id = ? AND active = 1",
                    board_id
                )
                .fetch_all(&self.pool)
                .await
            }
            ThreadStatus::Archived => {
                query_as!(
                    SelectionThread,
                    "SELECT * FROM threads WHERE board_id = ? AND archived = 1",
                    board_id
                )
                .fetch_all(&self.pool)
                .await
            }
            ThreadStatus::Inactive => {
                query_as!(
                    SelectionThread,
                    "SELECT * FROM threads WHERE board_id = ? AND active = 0 AND archived = 0",
                    board_id
                )
                .fetch_all(&self.pool)
                .await
            }
            ThreadStatus::Unarchived => {
                query_as!(
                    SelectionThread,
                    "SELECT * FROM threads WHERE board_id = ? AND archived = 0 ORDER BY sage_last_modified_at DESC",
                    board_id
                )
                .fetch_all(&self.pool)
                .await
            }
        }?;

        Ok(threads
            .into_iter()
            .map(SelectionThread::into_thread)
            .collect())
    }

    async fn get_threads_with_metadent(
        &self,
        board_id: Uuid,
    ) -> anyhow::Result<Vec<(Thread, ClientInfo, AuthedToken)>> {
        let threads = query_as!(
            SelectionThreadWithMetadent,
            r#"
                SELECT
                    t.id AS "id: Uuid",
                    t.board_id AS "board_id: Uuid",
                    t.thread_number AS thread_number,
                    t.last_modified_at AS last_modified_at,
                    t.sage_last_modified_at AS sage_last_modified_at,
                    t.title AS title,
                    t.authed_token_id AS "authed_token_id: Uuid",
                    t.metadent AS metadent,
                    t.response_count AS response_count,
                    t.no_pool AS "no_pool: bool",
                    t.active AS "active: bool",
                    t.archived AS "archived: bool",
                    t.archive_converted AS "archive_converted: bool",
                    (
                        SELECT r.client_info
                        FROM responses r
                        WHERE r.thread_id = t.id
                        AND r.res_order = 1
                    ) AS "client_info! : Json<ClientInfo>",
                    at.token AS token,
                    at.origin_ip AS origin_ip,
                    at.reduced_origin_ip AS reduced_origin_ip,
                    at.writing_ua AS writing_ua,
                    at.authed_ua AS authed_ua,
                    at.auth_code AS auth_code,
                    at.created_at AS created_at,
                    at.authed_at AS authed_at,
                    at.validity AS "validity: bool",
                    at.last_wrote_at AS last_wrote_at,
                    at.author_id_seed AS author_id_seed,
                    at.require_user_registration AS "require_user_registration: bool",
                    at.registered_user_id AS "registered_user_id?: Uuid",
                    at.require_reauth AS "require_reauth: bool"
                FROM
                    threads AS t
                INNER JOIN
                    authed_tokens AS at ON t.authed_token_id = at.id
                WHERE
                    t.board_id = ?
                    AND t.archived = 0
                ORDER BY
                    t.sage_last_modified_at DESC
"#,
            board_id
        )
        .fetch_all(&self.pool)
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
        let th = query_as!(
            SelectionThread,
            "SELECT * FROM threads
            WHERE thread_number = ?
            AND board_id = (
                SELECT id FROM boards WHERE board_key = ? LIMIT 1
            )",
            thread_number,
            board_key,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(th.map(SelectionThread::into_thread))
    }

    async fn create_thread(&self, thread: CreatingThread) -> anyhow::Result<()> {
        let metadent = Option::<&str>::from(thread.metadent);
        let metadent = metadent.unwrap_or("");

        let (response_id, thread_id, board_id) = (
            thread.response_id.as_bytes().to_vec(),
            thread.thread_id.as_bytes().to_vec(),
            thread.board_id.as_bytes().to_vec(),
        );
        let client_info_json = serde_json::to_string(&thread.client_info)?;

        let th_query = query!(
            r"INSERT INTO threads
                (
                    id,
                    board_id,
                    thread_number,
                    last_modified_at,
                    sage_last_modified_at,
                    title,
                    authed_token_id,
                    metadent,
                    response_count
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, 1)",
            thread_id,
            board_id,
            thread.unix_time as i64,
            thread.created_at,
            thread.created_at,
            thread.title,
            thread.authed_token_id.as_bytes().to_vec(),
            metadent
        );

        let res_query = query!(
            r"
            INSERT INTO responses
                (
                    id,
                    author_name,
                    mail,
                    author_id,
                    body,
                    thread_id,
                    board_id,
                    ip_addr,
                    authed_token_id,
                    created_at,
                    client_info,
                    res_order
                )
                VALUES
                (
                    ?, ?, ?, ?, ?,
                    ?, ?, ?, ?, ?,
                    ?, 1
                )",
            response_id,
            thread.name,
            thread.mail,
            thread.author_ch5id,
            thread.body,
            thread_id,
            board_id,
            thread.ip_addr,
            thread.authed_token_id.as_bytes().to_vec(),
            thread.created_at,
            client_info_json,
        );

        let mut tx = self.pool.begin().await?;
        th_query.execute(&mut *tx).await.map_err(|e| {
            if let Some(de) = e.as_database_error() {
                if de.is_unique_violation() {
                    anyhow::anyhow!("Given thread number is already exists")
                } else {
                    e.into()
                }
            } else {
                e.into()
            }
        })?;
        res_query.execute(&mut *tx).await?;
        tx.commit().await?;

        Ok(())
    }
}

#[derive(Debug)]
struct SelectionThread {
    id: Vec<u8>,
    board_id: Vec<u8>,
    thread_number: i64,
    last_modified_at: NaiveDateTime,
    sage_last_modified_at: NaiveDateTime,
    title: String,
    authed_token_id: Vec<u8>,
    metadent: String,
    response_count: i32,
    no_pool: i8,           // TINYINT
    active: i8,            // TINYINT
    archived: i8,          // TINYINT
    archive_converted: i8, // TINYINT
}

impl SelectionThread {
    fn into_thread(self) -> Thread {
        Thread {
            id: self.id.try_into().unwrap(),
            board_id: self.board_id.try_into().unwrap(),
            thread_number: self.thread_number,
            last_modified_at: Utc.from_utc_datetime(&self.last_modified_at),
            sage_last_modified_at: Utc.from_utc_datetime(&self.sage_last_modified_at),
            title: self.title,
            authed_token_id: self.authed_token_id.try_into().unwrap(),
            metadent: self.metadent,
            response_count: self.response_count as u32,
            no_pool: self.no_pool != 0,
            active: self.active != 0,
            archived: self.archived != 0,
        }
    }
}

#[derive(Debug)]
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
    no_pool: bool,           // TINYINT
    active: bool,            // TINYINT
    archived: bool,          // TINYINT
    archive_converted: bool, // TINYINT
    client_info: Json<ClientInfo>,

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
