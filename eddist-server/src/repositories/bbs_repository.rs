use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use eddist_core::domain::{
    board::{Board, BoardInfo},
    client_info::ClientInfo,
    ip_addr::{IpAddr, ReducedIpAddr},
    res::ResView,
};
use sqlx::{query, query_as, MySqlPool};
use uuid::Uuid;

use crate::domain::{
    authed_token::AuthedToken, cap::Cap, metadent::MetadentType, ng_word::NgWord, thread::Thread,
};

#[mockall::automock]
#[async_trait::async_trait]
pub trait BbsRepository: Send + Sync + 'static {
    async fn get_boards(&self) -> anyhow::Result<Vec<Board>>;
    async fn get_board(&self, board_key: &str) -> anyhow::Result<Option<Board>>;
    async fn get_board_info(&self, board_id: Uuid) -> anyhow::Result<Option<BoardInfo>>;
    async fn get_threads(
        &self,
        board_id: Uuid,
        status: ThreadStatus,
    ) -> anyhow::Result<Vec<Thread>>;
    async fn get_thread_by_board_key_and_thread_number(
        &self,
        board_key: &str,
        thread_number: u64,
    ) -> anyhow::Result<Option<Thread>>;
    async fn get_responses(&self, thread_id: Uuid) -> anyhow::Result<Vec<ResView>>;
    async fn get_authed_token(&self, token: &str) -> anyhow::Result<Option<AuthedToken>>;
    async fn get_authed_token_by_origin_ip_and_auth_code(
        &self,
        ip: &str,
        auth_code: &str,
    ) -> anyhow::Result<Option<AuthedToken>>;
    async fn create_thread(&self, thread: CreatingThread) -> anyhow::Result<()>;
    async fn create_response(&self, res: CreatingRes) -> anyhow::Result<()>;
    async fn create_authed_token(&self, authed_token: CreatingAuthedToken) -> anyhow::Result<()>;
    async fn activate_authed_status(
        &self,
        token: &str,
        authed_ua: &str,
        authed_time: DateTime<Utc>,
    ) -> anyhow::Result<()>;
    async fn revoke_authed_token(&self, token: &str) -> anyhow::Result<()>;

    async fn get_ng_words_by_board_key(&self, board_key: &str) -> anyhow::Result<Vec<NgWord>>;
    async fn get_cap_by_board_key(
        &self,
        cap_hash: &str,
        board_key: &str,
    ) -> anyhow::Result<Option<Cap>>;
}

#[derive(Debug, Clone)]
pub struct BbsRepositoryImpl {
    pool: MySqlPool,
}

impl BbsRepositoryImpl {
    pub fn new(pool: MySqlPool) -> BbsRepositoryImpl {
        BbsRepositoryImpl { pool }
    }
}

#[async_trait::async_trait]
impl BbsRepository for BbsRepositoryImpl {
    async fn get_boards(&self) -> anyhow::Result<Vec<Board>> {
        let boards = query_as!(
            Board,
            r#"
        SELECT
            id AS "id: Uuid",
            name,
            board_key,
            default_name
        FROM boards
        "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(boards)
    }

    async fn get_board(&self, board_key: &str) -> anyhow::Result<Option<Board>> {
        let query = query_as!(
            Board,
            r#"
        SELECT
            id AS "id: Uuid",
            name,
            board_key,
            default_name
        FROM boards 
        WHERE board_key = ?"#,
            board_key
        );

        let board = query.fetch_optional(&self.pool).await?;

        Ok(board.map(|x| Board {
            id: x.id,
            name: x.name,
            board_key: x.board_key,
            default_name: x.default_name,
        }))
    }

    async fn get_board_info(&self, board_id: Uuid) -> anyhow::Result<Option<BoardInfo>> {
        let query = query_as!(
            BoardInfo,
            r#"
        SELECT
            id AS "id: Uuid",
            local_rules,
            base_thread_creation_span_sec,
            base_response_creation_span_sec,
            max_thread_name_byte_length,
            max_author_name_byte_length,
            max_email_byte_length,
            max_response_body_byte_length,
            max_response_body_lines,
            threads_archive_cron,
            threads_archive_trigger_thread_count,
            created_at,
            updated_at
        FROM boards_info
        WHERE id = ?
        "#,
            board_id
        );

        Ok(query.fetch_optional(&self.pool).await?)
    }

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
                    "SELECT * FROM threads WHERE board_id = ? AND archived = 0",
                    board_id
                )
                .fetch_all(&self.pool)
                .await
            }
        }?;

        Ok(threads
            .into_iter()
            .map(|x| Thread {
                id: x.id.try_into().unwrap(),
                board_id: x.board_id.try_into().unwrap(),
                thread_number: x.thread_number,
                last_modified_at: Utc.from_utc_datetime(&x.last_modified_at),
                sage_last_modified_at: Utc.from_utc_datetime(&x.sage_last_modified_at),
                title: x.title,
                authed_token_id: x.authed_token_id.try_into().unwrap(),
                metadent: x.metadent,
                response_count: x.response_count as u32,
                no_pool: x.no_pool != 0,
                active: x.active != 0,
                archived: x.archived != 0,
            })
            .collect())
    }

    async fn get_thread_by_board_key_and_thread_number(
        &self,
        board_key: &str,
        thread_number: u64,
    ) -> anyhow::Result<Option<Thread>> {
        let query = query_as!(
            SelectionThread,
            "SELECT * FROM threads 
            WHERE thread_number = ? 
            AND board_id = (
                SELECT id FROM boards WHERE board_key = ? LIMIT 1
            )",
            thread_number,
            board_key,
        );

        let th = query.fetch_optional(&self.pool).await?;
        Ok(th.map(|th| Thread {
            id: th.id.try_into().unwrap(),
            board_id: th.board_id.try_into().unwrap(),
            thread_number: th.thread_number,
            last_modified_at: Utc.from_utc_datetime(&th.last_modified_at),
            sage_last_modified_at: Utc.from_utc_datetime(&th.sage_last_modified_at),
            title: th.title.clone(),
            authed_token_id: th.authed_token_id.try_into().unwrap(),
            metadent: th.metadent.clone(),
            response_count: th.response_count as u32,
            no_pool: th.no_pool != 0,
            active: th.active != 0,
            archived: th.archived != 0,
        }))
    }

    async fn get_responses(&self, thread_id: Uuid) -> anyhow::Result<Vec<ResView>> {
        let thread_id = Vec::<u8>::from(thread_id);

        let query = query_as!(
            SelectionRes,
            "SELECT
                author_name,
                mail,
                body,
                created_at,
                author_id,
                is_abone
            FROM responses WHERE thread_id = ? 
            ORDER BY res_order, id",
            thread_id
        );

        let responses = query.fetch_all(&self.pool).await?;

        Ok(responses
            .into_iter()
            .map(|x| ResView {
                author_name: x.author_name,
                mail: x.mail,
                body: x.body,
                created_at: Utc.from_utc_datetime(&x.created_at),
                author_id: x.author_id,
                is_abone: x.is_abone != 0,
            })
            .collect())
    }

    async fn get_authed_token(&self, token: &str) -> anyhow::Result<Option<AuthedToken>> {
        let query = query_as!(
            SelectionAuthedToken,
            "SELECT * FROM authed_tokens WHERE token = ?",
            token
        );

        let authed_token = query.fetch_optional(&self.pool).await?;

        Ok(authed_token.map(|x| AuthedToken {
            id: x.id.try_into().unwrap(),
            token: x.token,
            origin_ip: IpAddr::new(x.origin_ip.clone()),
            reduced_ip: ReducedIpAddr::from(x.reduced_origin_ip),
            writing_ua: x.writing_ua,
            authed_ua: x.authed_ua,
            auth_code: x.auth_code,
            created_at: x.created_at,
            authed_at: x.authed_at,
            validity: x.validity != 0,
        }))
    }

    async fn get_authed_token_by_origin_ip_and_auth_code(
        &self,
        reduced_ip: &str,
        auth_code: &str,
    ) -> anyhow::Result<Option<AuthedToken>> {
        let query = query_as!(
            SelectionAuthedToken,
            "SELECT * FROM authed_tokens WHERE reduced_origin_ip = ? AND auth_code = ?",
            reduced_ip,
            auth_code
        );

        let authed_token = query.fetch_optional(&self.pool).await?;

        Ok(authed_token.map(|x| AuthedToken {
            id: x.id.try_into().unwrap(),
            token: x.token,
            origin_ip: IpAddr::new(x.origin_ip.clone()),
            reduced_ip: ReducedIpAddr::from(x.origin_ip),
            writing_ua: x.writing_ua,
            authed_ua: x.authed_ua,
            auth_code: x.auth_code,
            created_at: x.created_at,
            authed_at: x.authed_at,
            validity: x.validity != 0,
        }))
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

    async fn create_response(&self, res: CreatingRes) -> anyhow::Result<()> {
        let (res_id, th_id, board_id) = (
            res.id.as_bytes().to_vec(),
            res.thread_id.as_bytes().to_vec(),
            res.board_id.as_bytes().to_vec(),
        );
        let client_info_json = serde_json::to_string(&res.client_info)?;

        let th_query = query!(
            "UPDATE threads SET
                last_modified_at = ?,
                response_count = response_count + 1,
                active = (
                CASE
                    WHEN response_count >= 1000 THEN 0
                    ELSE 1
                END
            )
            WHERE id = ?
        ",
            res.created_at,
            th_id,
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
                    ?, ?
                )",
            res_id,
            res.name,
            res.mail,
            res.author_ch5id,
            res.body,
            th_id,
            board_id,
            res.ip_addr,
            res.authed_token_id.as_bytes().to_vec(),
            res.created_at.clone(),
            client_info_json,
            res.res_order,
        );

        let mut tx = self.pool.begin().await?;
        th_query.execute(&mut *tx).await?;

        res_query.execute(&mut *tx).await?;
        tx.commit().await?;

        Ok(())
    }

    async fn create_authed_token(&self, authed_token: CreatingAuthedToken) -> anyhow::Result<()> {
        let ip_addr = authed_token.origin_ip.to_string();
        let reduced_ip = ReducedIpAddr::from(authed_token.origin_ip).to_string();

        let query = query!(
            "INSERT INTO authed_tokens 
                (
                    id,
                    token, 
                    origin_ip,
                    reduced_origin_ip,
                    writing_ua, 
                    auth_code, 
                    created_at, 
                    validity
                ) 
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            authed_token.id.as_bytes().to_vec(),
            authed_token.token,
            ip_addr,
            reduced_ip,
            authed_token.writing_ua,
            authed_token.auth_code,
            authed_token.created_at,
            false
        );

        query.execute(&self.pool).await?;

        Ok(())
    }

    async fn activate_authed_status(
        &self,
        token: &str,
        authed_ua: &str,
        authed_time: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let query = query!(
            "UPDATE authed_tokens SET validity = ?, authed_ua = ?, authed_at = ? WHERE token = ?",
            true,
            authed_ua,
            authed_time,
            token,
        );

        query.execute(&self.pool).await?;

        Ok(())
    }

    async fn revoke_authed_token(&self, token: &str) -> anyhow::Result<()> {
        let query = query!(
            "UPDATE authed_tokens SET validity = ? WHERE token = ?",
            false,
            token
        );

        query.execute(&self.pool).await?;

        Ok(())
    }

    async fn get_ng_words_by_board_key(&self, board_key: &str) -> anyhow::Result<Vec<NgWord>> {
        let ng_words = sqlx::query_as!(
            NgWord,
            r#"SELECT
                nw.id AS "id: Uuid",
                nw.name AS name,
                nw.word AS word,
                nw.created_at AS created_at,
                nw.updated_at AS updated_at
            FROM ng_words AS nw
            JOIN boards_ng_words AS bnw 
            ON nw.id = bnw.ng_word_id
            JOIN boards AS b
            ON bnw.board_id = b.id
            WHERE b.board_key = ?
        "#,
            board_key
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(ng_words)
    }

    async fn get_cap_by_board_key(
        &self,
        cap_hash: &str,
        board_key: &str,
    ) -> anyhow::Result<Option<Cap>> {
        let cap = sqlx::query_as!(
            Cap,
            r#"SELECT
                c.id AS "id: Uuid",
                c.name AS name,
                c.password_hash AS password_hash,
                c.description AS description,
                c.created_at AS created_at,
                c.updated_at AS updated_at
            FROM caps AS c
            JOIN boards_caps AS bc
            ON c.id = bc.cap_id
            JOIN boards AS b
            ON bc.board_id = b.id
            WHERE c.password_hash = ? AND b.board_key = ?
        "#,
            cap_hash,
            board_key
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(cap)
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

#[derive(Debug)]
struct SelectionRes {
    author_name: String,
    mail: String,
    body: String,
    created_at: NaiveDateTime,
    author_id: String,
    is_abone: i8, // TINYINT
}

#[derive(Debug)]
struct SelectionAuthedToken {
    id: Vec<u8>,
    token: String,
    origin_ip: String,
    reduced_origin_ip: String,
    writing_ua: String,
    authed_ua: Option<String>,
    auth_code: String,
    created_at: DateTime<Utc>,
    authed_at: Option<DateTime<Utc>>,
    validity: i8, // TINYINT
}

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

#[derive(Debug, Clone)]
pub struct CreatingThread {
    pub thread_id: Uuid,
    pub response_id: Uuid,
    pub title: String,
    pub unix_time: u64,
    pub body: String,
    pub name: String,
    pub mail: String,
    pub created_at: DateTime<Utc>,
    pub author_ch5id: String,
    pub authed_token_id: Uuid,
    pub ip_addr: String,
    pub board_id: Uuid,
    pub metadent: MetadentType,
    pub client_info: ClientInfo,
}

#[derive(Debug, Clone)]
pub struct CreatingRes {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub body: String,
    pub name: String,
    pub mail: String,
    pub author_ch5id: String,
    pub authed_token_id: Uuid,
    pub ip_addr: String,
    pub thread_id: Uuid,
    pub board_id: Uuid,
    pub client_info: ClientInfo,
    pub res_order: i32,
}

#[derive(Debug, Clone)]
pub struct CreatingAuthedToken {
    pub id: Uuid,
    pub token: String,
    pub origin_ip: IpAddr,
    pub writing_ua: String,
    pub auth_code: String,
    pub created_at: DateTime<Utc>,
}
