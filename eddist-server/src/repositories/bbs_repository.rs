use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use eddist_core::domain::{
    board::{Board, BoardInfo},
    cap::Cap,
    client_info::ClientInfo,
    ip_addr::{IpAddr, ReducedIpAddr},
    pubsub_repository::CreatingRes,
    res::ResView,
};
use serde::{Deserialize, Serialize};
#[cfg(not(feature = "backend-postgres"))]
use sqlx::{MySqlPool, query, query_as, types::Json};
#[cfg(feature = "backend-postgres")]
use sqlx::{PgPool, types::Json};
use uuid::Uuid;

use crate::domain::{
    authed_token::AuthedToken, metadent::MetadentType, ng_word::NgWord, thread::Thread,
};

// #[mockall::automock]
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
    async fn get_threads_with_metadent(
        &self,
        board_id: Uuid,
    ) -> anyhow::Result<Vec<(Thread, ClientInfo, AuthedToken)>>;
    async fn get_thread_by_board_key_and_thread_number(
        &self,
        board_key: &str,
        thread_number: u64,
    ) -> anyhow::Result<Option<Thread>>;
    async fn get_responses(&self, thread_id: Uuid) -> anyhow::Result<Vec<ResView>>;
    async fn get_authed_token(&self, token: &str) -> anyhow::Result<Option<AuthedToken>>;
    async fn get_authed_token_by_id(&self, id: Uuid) -> anyhow::Result<Option<AuthedToken>>;
    async fn get_authed_token_by_origin_ip_and_auth_code(
        &self,
        ip: &str,
        auth_code: &str,
    ) -> anyhow::Result<Option<AuthedToken>>;
    async fn get_unauthed_authed_token_by_auth_code(
        &self,
        auth_code: &str,
    ) -> anyhow::Result<Vec<AuthedToken>>;
    async fn create_thread(&self, thread: CreatingThread) -> anyhow::Result<()>;
    async fn create_response(&self, res: CreatingRes) -> anyhow::Result<()>;
    async fn create_authed_token(&self, authed_token: CreatingAuthedToken) -> anyhow::Result<()>;
    async fn activate_authed_status(
        &self,
        token: &str,
        authed_ua: &str,
        authed_time: DateTime<Utc>,
        additional_info: Option<serde_json::Value>,
    ) -> anyhow::Result<()>;
    async fn update_authed_token_last_wrote(
        &self,
        token_id: Uuid,
        last_wrote: DateTime<Utc>,
    ) -> anyhow::Result<()>;
    async fn revoke_authed_token(&self, token: &str) -> anyhow::Result<()>;
    async fn delete_authed_token(&self, token: &str) -> anyhow::Result<()>;
    async fn clear_require_reauth(&self, id: Uuid) -> anyhow::Result<()>;

    async fn get_ng_words_by_board_key(&self, board_key: &str) -> anyhow::Result<Vec<NgWord>>;
    async fn get_cap_by_board_key(
        &self,
        cap_hash: &str,
        board_key: &str,
    ) -> anyhow::Result<Option<Cap>>;
}

#[cfg(not(feature = "backend-postgres"))]
#[derive(Debug, Clone)]
pub struct BbsRepositoryImpl {
    pool: MySqlPool,
}

#[cfg(not(feature = "backend-postgres"))]
impl BbsRepositoryImpl {
    pub fn new(pool: MySqlPool) -> BbsRepositoryImpl {
        BbsRepositoryImpl { pool }
    }
}

#[cfg(not(feature = "backend-postgres"))]
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
            updated_at,
            read_only AS "read_only: bool",
            force_metadent_type
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
                    "SELECT * FROM threads WHERE board_id = ? AND archived = 0 ORDER BY sage_last_modified_at DESC",
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
            r#"SELECT
                id,
                token,
                origin_ip,
                reduced_origin_ip,
                asn_num,
                writing_ua,
                authed_ua,
                auth_code,
                created_at,
                authed_at,
                validity,
                last_wrote_at,
                author_id_seed,
                require_user_registration,
                registered_user_id,
                require_reauth
            FROM authed_tokens WHERE token = ?"#,
            token
        );

        let authed_token = query.fetch_optional(&self.pool).await?;

        Ok(authed_token.map(|x| AuthedToken {
            id: x.id.try_into().unwrap(),
            token: x.token,
            origin_ip: IpAddr::new(x.origin_ip.clone()),
            reduced_ip: ReducedIpAddr::from(x.reduced_origin_ip),
            asn_num: x.asn_num,
            writing_ua: x.writing_ua,
            authed_ua: x.authed_ua,
            auth_code: x.auth_code,
            created_at: x.created_at.and_utc(),
            authed_at: x.authed_at.map(|x| x.and_utc()),
            validity: x.validity != 0,
            last_wrote_at: x.last_wrote_at.map(|x| x.and_utc()),
            author_id_seed: x.author_id_seed,
            require_user_registration: x.require_user_registration != 0,
            registered_user_id: x.registered_user_id.map(|x| x.try_into().unwrap()),
            require_reauth: x.require_reauth != 0,
        }))
    }

    async fn get_authed_token_by_id(&self, id: Uuid) -> anyhow::Result<Option<AuthedToken>> {
        let query = query_as!(
            SelectionAuthedToken,
            r#"SELECT
                id,
                token,
                origin_ip,
                reduced_origin_ip,
                asn_num,
                writing_ua,
                authed_ua,
                auth_code,
                created_at,
                authed_at,
                validity,
                last_wrote_at,
                author_id_seed,
                require_user_registration,
                registered_user_id,
                require_reauth
            FROM authed_tokens WHERE id = ?"#,
            id.as_bytes().to_vec()
        );

        let authed_token = query.fetch_optional(&self.pool).await?;

        Ok(authed_token.map(|x| AuthedToken {
            id: x.id.try_into().unwrap(),
            token: x.token,
            origin_ip: IpAddr::new(x.origin_ip.clone()),
            reduced_ip: ReducedIpAddr::from(x.reduced_origin_ip),
            asn_num: x.asn_num,
            writing_ua: x.writing_ua,
            authed_ua: x.authed_ua,
            auth_code: x.auth_code,
            created_at: x.created_at.and_utc(),
            authed_at: x.authed_at.map(|x| x.and_utc()),
            validity: x.validity != 0,
            last_wrote_at: x.last_wrote_at.map(|x| x.and_utc()),
            author_id_seed: x.author_id_seed,
            require_user_registration: x.require_user_registration != 0,
            registered_user_id: x.registered_user_id.map(|x| x.try_into().unwrap()),
            require_reauth: x.require_reauth != 0,
        }))
    }

    async fn get_authed_token_by_origin_ip_and_auth_code(
        &self,
        reduced_ip: &str,
        auth_code: &str,
    ) -> anyhow::Result<Option<AuthedToken>> {
        let query = query_as!(
            SelectionAuthedToken,
            r#"SELECT
                id,
                token,
                origin_ip,
                reduced_origin_ip,
                asn_num,
                writing_ua,
                authed_ua,
                auth_code,
                created_at,
                authed_at,
                validity,
                last_wrote_at,
                author_id_seed,
                require_user_registration,
                registered_user_id,
                require_reauth
            FROM authed_tokens WHERE reduced_origin_ip = ? AND auth_code = ?"#,
            reduced_ip,
            auth_code
        );

        let authed_token = query.fetch_optional(&self.pool).await?;

        Ok(authed_token.map(|x| AuthedToken {
            id: x.id.try_into().unwrap(),
            token: x.token,
            origin_ip: IpAddr::new(x.origin_ip.clone()),
            reduced_ip: ReducedIpAddr::from(x.origin_ip),
            asn_num: x.asn_num,
            writing_ua: x.writing_ua,
            authed_ua: x.authed_ua,
            auth_code: x.auth_code,
            created_at: x.created_at.and_utc(),
            authed_at: x.authed_at.map(|x| x.and_utc()),
            validity: x.validity != 0,
            last_wrote_at: x.last_wrote_at.map(|x| x.and_utc()),
            author_id_seed: x.author_id_seed,
            require_user_registration: x.require_user_registration != 0,
            registered_user_id: x.registered_user_id.map(|x| x.try_into().unwrap()),
            require_reauth: x.require_reauth != 0,
        }))
    }

    async fn get_unauthed_authed_token_by_auth_code(
        &self,
        auth_code: &str,
    ) -> anyhow::Result<Vec<AuthedToken>> {
        let query = query_as!(
            SelectionAuthedToken,
            r#"SELECT
                id,
                token,
                origin_ip,
                reduced_origin_ip,
                asn_num,
                writing_ua,
                authed_ua,
                auth_code,
                created_at,
                authed_at,
                validity,
                last_wrote_at,
                author_id_seed,
                require_user_registration,
                registered_user_id,
                require_reauth
            FROM authed_tokens WHERE auth_code = ? AND validity = false"#,
            auth_code
        );

        let authed_tokens = query.fetch_all(&self.pool).await?;

        Ok(authed_tokens
            .into_iter()
            .map(|x| AuthedToken {
                id: x.id.try_into().unwrap(),
                token: x.token,
                origin_ip: IpAddr::new(x.origin_ip.clone()),
                reduced_ip: ReducedIpAddr::from(x.reduced_origin_ip),
                asn_num: x.asn_num,
                writing_ua: x.writing_ua,
                authed_ua: x.authed_ua,
                auth_code: x.auth_code,
                created_at: x.created_at.and_utc(),
                authed_at: x.authed_at.map(|x| x.and_utc()),
                validity: x.validity != 0,
                last_wrote_at: x.last_wrote_at.map(|x| x.and_utc()),
                author_id_seed: x.author_id_seed,
                require_user_registration: x.require_user_registration != 0,
                registered_user_id: x.registered_user_id.map(|x| x.try_into().unwrap()),
                require_reauth: x.require_reauth != 0,
            })
            .collect())
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
                sage_last_modified_at = (
                    CASE
                        WHEN ? THEN sage_last_modified_at
                        ELSE ?
                    END
                ),
                active = (
                    CASE
                        WHEN response_count >= 1000 THEN 0
                        ELSE 1
                    END
                )
            WHERE id = ?
        ",
            res.created_at,
            res.is_sage,
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
                    asn_num,
                    writing_ua,
                    auth_code,
                    created_at,
                    validity,
                    author_id_seed,
                    require_user_registration
                )
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            authed_token.id.as_bytes().to_vec(),
            authed_token.token,
            ip_addr,
            reduced_ip,
            authed_token.asn_num,
            authed_token.writing_ua,
            authed_token.auth_code,
            authed_token.created_at,
            false,
            authed_token.author_id_seed,
            authed_token.require_user_registration,
        );

        query.execute(&self.pool).await?;

        Ok(())
    }

    async fn activate_authed_status(
        &self,
        token: &str,
        authed_ua: &str,
        authed_time: DateTime<Utc>,
        additional_info: Option<serde_json::Value>,
    ) -> anyhow::Result<()> {
        let additional_info_json = additional_info.and_then(|v| serde_json::to_string(&v).ok());

        let query = query!(
            "UPDATE authed_tokens SET validity = ?, authed_ua = ?, authed_at = ?, additional_info = ? WHERE token = ?",
            true,
            authed_ua,
            authed_time,
            additional_info_json,
            token,
        );

        query.execute(&self.pool).await?;

        Ok(())
    }

    async fn update_authed_token_last_wrote(
        &self,
        token_id: Uuid,
        last_wrote: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let query = query!(
            "UPDATE authed_tokens SET last_wrote_at = ? WHERE id = ?",
            last_wrote,
            token_id.as_bytes().to_vec(),
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

    async fn delete_authed_token(&self, token: &str) -> anyhow::Result<()> {
        let query = query!("DELETE FROM authed_tokens WHERE token = ?", token);

        query.execute(&self.pool).await?;

        Ok(())
    }

    async fn clear_require_reauth(&self, id: Uuid) -> anyhow::Result<()> {
        query!(
            "UPDATE authed_tokens SET require_reauth = 0 WHERE id = ?",
            id.as_bytes().to_vec()
        )
        .execute(&self.pool)
        .await?;
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

#[cfg(not(feature = "backend-postgres"))]
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

#[cfg(not(feature = "backend-postgres"))]
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

#[cfg(not(feature = "backend-postgres"))]
#[derive(Debug)]
struct SelectionRes {
    author_name: String,
    mail: String,
    body: String,
    created_at: NaiveDateTime,
    author_id: String,
    is_abone: i8, // TINYINT
}

#[cfg(not(feature = "backend-postgres"))]
#[derive(Debug)]
struct SelectionAuthedToken {
    id: Vec<u8>,
    token: String,
    origin_ip: String,
    reduced_origin_ip: String,
    asn_num: i32,
    writing_ua: String,
    authed_ua: Option<String>,
    auth_code: String,
    created_at: NaiveDateTime,
    authed_at: Option<NaiveDateTime>,
    validity: i8, // TINYINT
    last_wrote_at: Option<NaiveDateTime>,
    author_id_seed: Vec<u8>,
    require_user_registration: i8, // TINYINT
    registered_user_id: Option<Vec<u8>>,
    require_reauth: i8, // TINYINT
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub struct CreatingAuthedToken {
    pub id: Uuid,
    pub token: String,
    pub origin_ip: IpAddr,
    pub asn_num: i32,
    pub writing_ua: String,
    pub auth_code: String,
    pub created_at: DateTime<Utc>,
    pub author_id_seed: Vec<u8>,
    pub require_user_registration: bool,
}

// ===== PostgreSQL implementation =====

#[cfg(feature = "backend-postgres")]
#[derive(Debug, sqlx::FromRow)]
struct BoardPg {
    pub id: Uuid,
    pub name: String,
    pub board_key: String,
    pub default_name: String,
}

#[cfg(feature = "backend-postgres")]
#[derive(Debug, sqlx::FromRow)]
struct BoardInfoPg {
    pub id: Uuid,
    pub local_rules: String,
    pub base_thread_creation_span_sec: i32,
    pub base_response_creation_span_sec: i32,
    pub max_thread_name_byte_length: i32,
    pub max_author_name_byte_length: i32,
    pub max_email_byte_length: i32,
    pub max_response_body_byte_length: i32,
    pub max_response_body_lines: i32,
    pub threads_archive_cron: Option<String>,
    pub threads_archive_trigger_thread_count: Option<i32>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
    pub read_only: bool,
    pub force_metadent_type: Option<String>,
}

#[cfg(feature = "backend-postgres")]
#[derive(Debug, sqlx::FromRow)]
struct ThreadPg {
    pub id: Uuid,
    pub board_id: Uuid,
    pub thread_number: i64,
    pub last_modified_at: chrono::DateTime<Utc>,
    pub sage_last_modified_at: chrono::DateTime<Utc>,
    pub title: String,
    pub authed_token_id: Uuid,
    pub metadent: String,
    pub response_count: i32,
    pub no_pool: bool,
    pub active: bool,
    pub archived: bool,
}

#[cfg(feature = "backend-postgres")]
#[derive(Debug, sqlx::FromRow)]
struct ThreadWithMetadentPg {
    pub id: Uuid,
    pub board_id: Uuid,
    pub thread_number: i64,
    pub last_modified_at: chrono::DateTime<Utc>,
    pub sage_last_modified_at: chrono::DateTime<Utc>,
    pub title: String,
    pub authed_token_id: Uuid,
    pub metadent: String,
    pub response_count: i32,
    pub no_pool: bool,
    pub active: bool,
    pub archived: bool,
    pub client_info: Json<ClientInfo>,
    pub token: String,
    pub origin_ip: String,
    pub reduced_origin_ip: String,
    pub writing_ua: String,
    pub authed_ua: Option<String>,
    pub auth_code: String,
    pub at_created_at: chrono::DateTime<Utc>,
    pub authed_at: Option<chrono::DateTime<Utc>>,
    pub validity: bool,
    pub last_wrote_at: Option<chrono::DateTime<Utc>>,
    pub author_id_seed: Vec<u8>,
    pub require_user_registration: bool,
    pub registered_user_id: Option<Uuid>,
    pub require_reauth: bool,
}

#[cfg(feature = "backend-postgres")]
#[derive(Debug, sqlx::FromRow)]
struct ResPg {
    pub author_name: String,
    pub mail: String,
    pub body: String,
    pub created_at: chrono::DateTime<Utc>,
    pub author_id: String,
    pub is_abone: bool,
}

#[cfg(feature = "backend-postgres")]
#[derive(Debug, sqlx::FromRow)]
struct AuthedTokenPg {
    pub id: Uuid,
    pub token: String,
    pub origin_ip: String,
    pub reduced_origin_ip: String,
    pub asn_num: i32,
    pub writing_ua: String,
    pub authed_ua: Option<String>,
    pub auth_code: String,
    pub created_at: chrono::DateTime<Utc>,
    pub authed_at: Option<chrono::DateTime<Utc>>,
    pub validity: bool,
    pub last_wrote_at: Option<chrono::DateTime<Utc>>,
    pub author_id_seed: Vec<u8>,
    pub require_user_registration: bool,
    pub registered_user_id: Option<Uuid>,
    pub require_reauth: bool,
}

#[cfg(feature = "backend-postgres")]
impl From<AuthedTokenPg> for AuthedToken {
    fn from(x: AuthedTokenPg) -> Self {
        AuthedToken {
            id: x.id,
            token: x.token,
            origin_ip: IpAddr::new(x.origin_ip.clone()),
            reduced_ip: ReducedIpAddr::from(x.reduced_origin_ip),
            asn_num: x.asn_num,
            writing_ua: x.writing_ua,
            authed_ua: x.authed_ua,
            auth_code: x.auth_code,
            created_at: x.created_at,
            authed_at: x.authed_at,
            validity: x.validity,
            last_wrote_at: x.last_wrote_at,
            author_id_seed: x.author_id_seed,
            require_user_registration: x.require_user_registration,
            registered_user_id: x.registered_user_id,
            require_reauth: x.require_reauth,
        }
    }
}

#[cfg(feature = "backend-postgres")]
#[derive(Debug, sqlx::FromRow)]
struct NgWordPg {
    pub id: Uuid,
    pub name: String,
    pub word: String,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

#[cfg(feature = "backend-postgres")]
#[derive(Debug, sqlx::FromRow)]
struct CapPg {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub password_hash: String,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

#[cfg(feature = "backend-postgres")]
#[derive(Debug, Clone)]
pub struct BbsRepositoryPgImpl {
    pool: PgPool,
}

#[cfg(feature = "backend-postgres")]
impl BbsRepositoryPgImpl {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[cfg(feature = "backend-postgres")]
#[async_trait::async_trait]
impl BbsRepository for BbsRepositoryPgImpl {
    async fn get_boards(&self) -> anyhow::Result<Vec<Board>> {
        let rows =
            sqlx::query_as::<_, BoardPg>("SELECT id, name, board_key, default_name FROM boards")
                .fetch_all(&self.pool)
                .await?;

        Ok(rows
            .into_iter()
            .map(|r| Board {
                id: r.id,
                name: r.name,
                board_key: r.board_key,
                default_name: r.default_name,
            })
            .collect::<Vec<_>>())
    }

    async fn get_board(&self, board_key: &str) -> anyhow::Result<Option<Board>> {
        let row = sqlx::query_as::<_, BoardPg>(
            "SELECT id, name, board_key, default_name FROM boards WHERE board_key = $1",
        )
        .bind(board_key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| Board {
            id: r.id,
            name: r.name,
            board_key: r.board_key,
            default_name: r.default_name,
        }))
    }

    async fn get_board_info(&self, board_id: Uuid) -> anyhow::Result<Option<BoardInfo>> {
        let row = sqlx::query_as::<_, BoardInfoPg>(
            r#"
            SELECT id, local_rules, base_thread_creation_span_sec, base_response_creation_span_sec,
                   max_thread_name_byte_length, max_author_name_byte_length, max_email_byte_length,
                   max_response_body_byte_length, max_response_body_lines, threads_archive_cron,
                   threads_archive_trigger_thread_count, created_at, updated_at, read_only, force_metadent_type
            FROM boards_info
            WHERE id = $1
            "#,
        )
        .bind(board_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| BoardInfo {
            id: r.id,
            local_rules: r.local_rules,
            base_thread_creation_span_sec: r.base_thread_creation_span_sec,
            base_response_creation_span_sec: r.base_response_creation_span_sec,
            max_thread_name_byte_length: r.max_thread_name_byte_length,
            max_author_name_byte_length: r.max_author_name_byte_length,
            max_email_byte_length: r.max_email_byte_length,
            max_response_body_byte_length: r.max_response_body_byte_length,
            max_response_body_lines: r.max_response_body_lines,
            threads_archive_cron: r.threads_archive_cron,
            threads_archive_trigger_thread_count: r.threads_archive_trigger_thread_count,
            created_at: r.created_at.naive_utc(),
            updated_at: r.updated_at.naive_utc(),
            read_only: r.read_only,
            force_metadent_type: r.force_metadent_type,
        }))
    }

    async fn get_threads(
        &self,
        board_id: Uuid,
        status: ThreadStatus,
    ) -> anyhow::Result<Vec<crate::domain::thread::Thread>> {
        let sql = match status {
            ThreadStatus::Active => {
                "SELECT id, board_id, thread_number, last_modified_at, sage_last_modified_at, title, authed_token_id, metadent, response_count, no_pool, active, archived FROM threads WHERE board_id = $1 AND active = TRUE"
            }
            ThreadStatus::Archived => {
                "SELECT id, board_id, thread_number, last_modified_at, sage_last_modified_at, title, authed_token_id, metadent, response_count, no_pool, active, archived FROM threads WHERE board_id = $1 AND archived = TRUE"
            }
            ThreadStatus::Inactive => {
                "SELECT id, board_id, thread_number, last_modified_at, sage_last_modified_at, title, authed_token_id, metadent, response_count, no_pool, active, archived FROM threads WHERE board_id = $1 AND active = FALSE AND archived = FALSE"
            }
            ThreadStatus::Unarchived => {
                "SELECT id, board_id, thread_number, last_modified_at, sage_last_modified_at, title, authed_token_id, metadent, response_count, no_pool, active, archived FROM threads WHERE board_id = $1 AND archived = FALSE ORDER BY sage_last_modified_at DESC"
            }
        };

        let rows = sqlx::query_as::<_, ThreadPg>(sql)
            .bind(board_id)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows
            .into_iter()
            .map(|r| crate::domain::thread::Thread {
                id: r.id,
                board_id: r.board_id,
                thread_number: r.thread_number,
                last_modified_at: r.last_modified_at,
                sage_last_modified_at: r.sage_last_modified_at,
                title: r.title,
                authed_token_id: r.authed_token_id,
                metadent: r.metadent,
                response_count: r.response_count as u32,
                no_pool: r.no_pool,
                active: r.active,
                archived: r.archived,
            })
            .collect::<Vec<_>>())
    }

    async fn get_threads_with_metadent(
        &self,
        board_id: Uuid,
    ) -> anyhow::Result<Vec<(crate::domain::thread::Thread, ClientInfo, AuthedToken)>> {
        let rows = sqlx::query_as::<_, ThreadWithMetadentPg>(
            r#"
            SELECT
                t.id, t.board_id, t.thread_number, t.last_modified_at, t.sage_last_modified_at,
                t.title, t.authed_token_id, t.metadent, t.response_count, t.no_pool, t.active, t.archived,
                (
                    SELECT r.client_info
                    FROM responses r
                    WHERE r.thread_id = t.id AND r.res_order = 1
                ) AS client_info,
                at.token, at.origin_ip, at.reduced_origin_ip, at.writing_ua, at.authed_ua, at.auth_code,
                at.created_at AS at_created_at, at.authed_at, at.validity, at.last_wrote_at,
                at.author_id_seed, at.require_user_registration, at.registered_user_id, at.require_reauth
            FROM threads AS t
            INNER JOIN authed_tokens AS at ON t.authed_token_id = at.id
            WHERE t.board_id = $1 AND t.archived = FALSE
            ORDER BY t.sage_last_modified_at DESC
            "#,
        )
        .bind(board_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|x| {
                (
                    crate::domain::thread::Thread {
                        id: x.id,
                        board_id: x.board_id,
                        thread_number: x.thread_number,
                        last_modified_at: x.last_modified_at,
                        sage_last_modified_at: x.sage_last_modified_at,
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
                        origin_ip: IpAddr::new(x.origin_ip.clone()),
                        reduced_ip: ReducedIpAddr::from(x.reduced_origin_ip),
                        asn_num: 0,
                        writing_ua: x.writing_ua,
                        authed_ua: x.authed_ua,
                        auth_code: x.auth_code,
                        created_at: x.at_created_at,
                        authed_at: x.authed_at,
                        validity: x.validity,
                        last_wrote_at: x.last_wrote_at,
                        author_id_seed: x.author_id_seed,
                        require_user_registration: x.require_user_registration,
                        registered_user_id: x.registered_user_id,
                        require_reauth: x.require_reauth,
                    },
                )
            })
            .collect::<Vec<_>>())
    }

    async fn get_thread_by_board_key_and_thread_number(
        &self,
        board_key: &str,
        thread_number: u64,
    ) -> anyhow::Result<Option<crate::domain::thread::Thread>> {
        let row = sqlx::query_as::<_, ThreadPg>(
            r#"
            SELECT id, board_id, thread_number, last_modified_at, sage_last_modified_at,
                   title, authed_token_id, metadent, response_count, no_pool, active, archived
            FROM threads
            WHERE thread_number = $1
              AND board_id = (SELECT id FROM boards WHERE board_key = $2 LIMIT 1)
            "#,
        )
        .bind(thread_number as i64)
        .bind(board_key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| crate::domain::thread::Thread {
            id: r.id,
            board_id: r.board_id,
            thread_number: r.thread_number,
            last_modified_at: r.last_modified_at,
            sage_last_modified_at: r.sage_last_modified_at,
            title: r.title,
            authed_token_id: r.authed_token_id,
            metadent: r.metadent,
            response_count: r.response_count as u32,
            no_pool: r.no_pool,
            active: r.active,
            archived: r.archived,
        }))
    }

    async fn get_responses(&self, thread_id: Uuid) -> anyhow::Result<Vec<ResView>> {
        let rows = sqlx::query_as::<_, ResPg>(
            r#"
            SELECT author_name, mail, body, created_at, author_id, is_abone
            FROM responses
            WHERE thread_id = $1
            ORDER BY res_order, id
            "#,
        )
        .bind(thread_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ResView {
                author_name: r.author_name,
                mail: r.mail,
                body: r.body,
                created_at: r.created_at,
                author_id: r.author_id,
                is_abone: r.is_abone,
            })
            .collect::<Vec<_>>())
    }

    async fn get_authed_token(&self, token: &str) -> anyhow::Result<Option<AuthedToken>> {
        let row = sqlx::query_as::<_, AuthedTokenPg>(
            r#"
            SELECT id, token, origin_ip, reduced_origin_ip, asn_num, writing_ua, authed_ua,
                   auth_code, created_at, authed_at, validity, last_wrote_at, author_id_seed,
                   require_user_registration, registered_user_id, require_reauth
            FROM authed_tokens WHERE token = $1
            "#,
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(AuthedToken::from))
    }

    async fn get_authed_token_by_id(&self, id: Uuid) -> anyhow::Result<Option<AuthedToken>> {
        let row = sqlx::query_as::<_, AuthedTokenPg>(
            r#"
            SELECT id, token, origin_ip, reduced_origin_ip, asn_num, writing_ua, authed_ua,
                   auth_code, created_at, authed_at, validity, last_wrote_at, author_id_seed,
                   require_user_registration, registered_user_id, require_reauth
            FROM authed_tokens WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(AuthedToken::from))
    }

    async fn get_authed_token_by_origin_ip_and_auth_code(
        &self,
        reduced_ip: &str,
        auth_code: &str,
    ) -> anyhow::Result<Option<AuthedToken>> {
        let row = sqlx::query_as::<_, AuthedTokenPg>(
            r#"
            SELECT id, token, origin_ip, reduced_origin_ip, asn_num, writing_ua, authed_ua,
                   auth_code, created_at, authed_at, validity, last_wrote_at, author_id_seed,
                   require_user_registration, registered_user_id, require_reauth
            FROM authed_tokens WHERE reduced_origin_ip = $1 AND auth_code = $2
            "#,
        )
        .bind(reduced_ip)
        .bind(auth_code)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|x| {
            let origin_ip = x.origin_ip.clone();
            AuthedToken {
                id: x.id,
                token: x.token,
                origin_ip: IpAddr::new(origin_ip.clone()),
                reduced_ip: ReducedIpAddr::from(origin_ip),
                asn_num: x.asn_num,
                writing_ua: x.writing_ua,
                authed_ua: x.authed_ua,
                auth_code: x.auth_code,
                created_at: x.created_at,
                authed_at: x.authed_at,
                validity: x.validity,
                last_wrote_at: x.last_wrote_at,
                author_id_seed: x.author_id_seed,
                require_user_registration: x.require_user_registration,
                registered_user_id: x.registered_user_id,
                require_reauth: x.require_reauth,
            }
        }))
    }

    async fn get_unauthed_authed_token_by_auth_code(
        &self,
        auth_code: &str,
    ) -> anyhow::Result<Vec<AuthedToken>> {
        let rows = sqlx::query_as::<_, AuthedTokenPg>(
            r#"
            SELECT id, token, origin_ip, reduced_origin_ip, asn_num, writing_ua, authed_ua,
                   auth_code, created_at, authed_at, validity, last_wrote_at, author_id_seed,
                   require_user_registration, registered_user_id, require_reauth
            FROM authed_tokens WHERE auth_code = $1 AND validity = FALSE
            "#,
        )
        .bind(auth_code)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(AuthedToken::from).collect::<Vec<_>>())
    }

    async fn create_thread(&self, thread: CreatingThread) -> anyhow::Result<()> {
        let metadent = Option::<&str>::from(thread.metadent).unwrap_or("");
        let client_info_json = serde_json::to_value(&thread.client_info)?;

        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO threads
                (id, board_id, thread_number, last_modified_at, sage_last_modified_at,
                 title, authed_token_id, metadent, response_count)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 1)
            "#,
        )
        .bind(thread.thread_id)
        .bind(thread.board_id)
        .bind(thread.unix_time as i64)
        .bind(thread.created_at)
        .bind(thread.created_at)
        .bind(&thread.title)
        .bind(thread.authed_token_id)
        .bind(metadent)
        .execute(&mut *tx)
        .await
        .map_err(|e| {
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

        sqlx::query(
            r#"
            INSERT INTO responses
                (id, author_name, mail, author_id, body, thread_id, board_id,
                 ip_addr, authed_token_id, created_at, client_info, res_order)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 1)
            "#,
        )
        .bind(thread.response_id)
        .bind(&thread.name)
        .bind(&thread.mail)
        .bind(&thread.author_ch5id)
        .bind(&thread.body)
        .bind(thread.thread_id)
        .bind(thread.board_id)
        .bind(&thread.ip_addr)
        .bind(thread.authed_token_id)
        .bind(thread.created_at)
        .bind(client_info_json)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn create_response(&self, res: CreatingRes) -> anyhow::Result<()> {
        let client_info_json = serde_json::to_value(&res.client_info)?;

        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            UPDATE threads SET
                last_modified_at = $1,
                response_count = response_count + 1,
                sage_last_modified_at = CASE WHEN $2 THEN sage_last_modified_at ELSE $3 END,
                active = CASE WHEN response_count >= 1000 THEN FALSE ELSE TRUE END
            WHERE id = $4
            "#,
        )
        .bind(res.created_at)
        .bind(res.is_sage)
        .bind(res.created_at)
        .bind(res.thread_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO responses
                (id, author_name, mail, author_id, body, thread_id, board_id,
                 ip_addr, authed_token_id, created_at, client_info, res_order)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
        )
        .bind(res.id)
        .bind(&res.name)
        .bind(&res.mail)
        .bind(&res.author_ch5id)
        .bind(&res.body)
        .bind(res.thread_id)
        .bind(res.board_id)
        .bind(&res.ip_addr)
        .bind(res.authed_token_id)
        .bind(res.created_at)
        .bind(client_info_json)
        .bind(res.res_order)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn create_authed_token(&self, authed_token: CreatingAuthedToken) -> anyhow::Result<()> {
        let ip_addr = authed_token.origin_ip.to_string();
        let reduced_ip = ReducedIpAddr::from(authed_token.origin_ip).to_string();

        sqlx::query(
            r#"
            INSERT INTO authed_tokens
                (id, token, origin_ip, reduced_origin_ip, asn_num, writing_ua, auth_code,
                 created_at, validity, author_id_seed, require_user_registration)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, FALSE, $9, $10)
            "#,
        )
        .bind(authed_token.id)
        .bind(&authed_token.token)
        .bind(&ip_addr)
        .bind(&reduced_ip)
        .bind(authed_token.asn_num)
        .bind(&authed_token.writing_ua)
        .bind(&authed_token.auth_code)
        .bind(authed_token.created_at)
        .bind(&authed_token.author_id_seed)
        .bind(authed_token.require_user_registration)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn activate_authed_status(
        &self,
        token: &str,
        authed_ua: &str,
        authed_time: DateTime<Utc>,
        additional_info: Option<serde_json::Value>,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE authed_tokens SET validity = TRUE, authed_ua = $1, authed_at = $2, additional_info = $3 WHERE token = $4",
        )
        .bind(authed_ua)
        .bind(authed_time)
        .bind(additional_info)
        .bind(token)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update_authed_token_last_wrote(
        &self,
        token_id: Uuid,
        last_wrote: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        sqlx::query("UPDATE authed_tokens SET last_wrote_at = $1 WHERE id = $2")
            .bind(last_wrote)
            .bind(token_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn revoke_authed_token(&self, token: &str) -> anyhow::Result<()> {
        sqlx::query("UPDATE authed_tokens SET validity = FALSE WHERE token = $1")
            .bind(token)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn delete_authed_token(&self, token: &str) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM authed_tokens WHERE token = $1")
            .bind(token)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn clear_require_reauth(&self, id: Uuid) -> anyhow::Result<()> {
        sqlx::query("UPDATE authed_tokens SET require_reauth = FALSE WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn get_ng_words_by_board_key(
        &self,
        board_key: &str,
    ) -> anyhow::Result<Vec<crate::domain::ng_word::NgWord>> {
        let rows = sqlx::query_as::<_, NgWordPg>(
            r#"
            SELECT nw.id, nw.name, nw.word, nw.created_at, nw.updated_at
            FROM ng_words AS nw
            JOIN boards_ng_words AS bnw ON nw.id = bnw.ng_word_id
            JOIN boards AS b ON bnw.board_id = b.id
            WHERE b.board_key = $1
            "#,
        )
        .bind(board_key)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| crate::domain::ng_word::NgWord {
                id: r.id,
                name: r.name,
                word: r.word,
                created_at: r.created_at.naive_utc(),
                updated_at: r.updated_at.naive_utc(),
            })
            .collect::<Vec<_>>())
    }

    async fn get_cap_by_board_key(
        &self,
        cap_hash: &str,
        board_key: &str,
    ) -> anyhow::Result<Option<Cap>> {
        let row = sqlx::query_as::<_, CapPg>(
            r#"
            SELECT c.id, c.name, c.description, c.password_hash, c.created_at, c.updated_at
            FROM caps AS c
            JOIN boards_caps AS bc ON c.id = bc.cap_id
            JOIN boards AS b ON bc.board_id = b.id
            WHERE c.password_hash = $1 AND b.board_key = $2
            "#,
        )
        .bind(cap_hash)
        .bind(board_key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| Cap {
            id: r.id,
            name: r.name,
            description: r.description,
            password_hash: r.password_hash,
            created_at: r.created_at.naive_utc(),
            updated_at: r.updated_at.naive_utc(),
        }))
    }
}
