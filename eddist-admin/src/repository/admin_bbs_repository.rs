use std::{collections::HashMap, vec};

use chrono::{TimeZone, Utc};
use eddist_core::domain::client_info::ClientInfo;
use sqlx::{query, query_as, types::Json, Executor, FromRow, MySqlPool};
use uuid::Uuid;

use crate::{AuthedToken, Board, CreateBoardInput, NgWord, Res, Thread};

#[async_trait::async_trait]
pub trait AdminBbsRepository: Send + Sync {
    async fn get_boards_by_key(&self, keys: Option<Vec<String>>) -> anyhow::Result<Vec<Board>>;
    async fn get_threads_by_thread_id(
        &self,
        board_key: &str,
        thread_numbers: Option<Vec<u64>>,
    ) -> anyhow::Result<Vec<Thread>>;
    async fn get_archived_threads_by_thread_id(
        &self,
        board_key: &str,
        thread_numbers: Option<Vec<u64>>,
    ) -> anyhow::Result<Vec<Thread>>;
    async fn get_archived_threads_by_filter(
        &self,
        board_key: &str,
        keyword: Option<&str>,
        range: (Option<chrono::DateTime<Utc>>, Option<chrono::DateTime<Utc>>),
        page: u64,
        limit: u64, // <= 100
    ) -> anyhow::Result<Vec<Thread>>;

    async fn create_board(&self, board: CreateBoardInput) -> anyhow::Result<Board>;

    async fn get_reses_by_thread_id(
        &self,
        board_key: &str,
        thread_number: u64,
    ) -> anyhow::Result<Vec<Res>>;
    async fn get_archived_reses_by_thread_id(
        &self,
        board_key: &str,
        thread_number: u64,
    ) -> anyhow::Result<Vec<Res>>;
    async fn get_res(
        &self,
        res_id: Uuid,
    ) -> anyhow::Result<(Res, String, String, u64, Option<String>)>;

    async fn update_res(
        &self,
        id: Uuid,
        author_name: Option<String>,
        mail: Option<String>,
        body: Option<String>,
        is_abone: Option<bool>,
    ) -> anyhow::Result<Res>;

    async fn get_authed_token(&self, id: Uuid) -> anyhow::Result<AuthedToken>;
    async fn delete_authed_token(&self, id: Uuid) -> anyhow::Result<()>;
    async fn delete_authed_token_by_origin_ip(&self, id: Uuid) -> anyhow::Result<()>;

    async fn get_ng_words(&self) -> anyhow::Result<Vec<NgWord>>;
    async fn update_ng_word(
        &self,
        id: Uuid,
        name: Option<&str>,
        word: Option<&str>,
        board_ids: Option<Vec<Uuid>>,
    ) -> anyhow::Result<NgWord>;
    async fn create_ng_word(&self, name: &str, word: &str) -> anyhow::Result<NgWord>;
    async fn delete_ng_word(&self, ng_word_id: Uuid) -> anyhow::Result<()>;
}

#[derive(Clone)]
pub struct AdminBbsRepositoryImpl(MySqlPool);

impl AdminBbsRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self(pool)
    }
}

#[derive(Debug, FromRow)]
pub struct SelectionBoardWithThreadCount {
    pub id: Vec<u8>,
    pub name: String,
    pub board_key: String,
    pub default_name: String,
    pub thread_count: i64,
}

#[derive(Debug, FromRow)]
pub struct SelectionThread {
    pub id: Vec<u8>,
    pub board_id: Vec<u8>,
    pub thread_number: i64,
    pub last_modified_at: chrono::DateTime<Utc>,
    pub sage_last_modified_at: chrono::DateTime<Utc>,
    pub title: String,
    pub authed_token_id: Vec<u8>,
    pub metadent: String,
    pub response_count: i32,
    pub no_pool: bool,
    pub archived: bool,
    pub active: bool,
}

#[derive(Debug)]
pub struct SelectionRes {
    pub id: Vec<u8>,
    pub author_name: String,
    pub mail: String,
    pub body: String,
    pub created_at: chrono::NaiveDateTime,
    pub author_id: String,
    pub ip_addr: String,
    pub authed_token_id: Vec<u8>,
    pub board_id: Vec<u8>,
    pub thread_id: Vec<u8>,
    pub is_abone: i8,
    pub res_order: i32,
    pub client_info: Json<ClientInfo>,
}

#[derive(Debug)]
pub struct SelectionNgWord {
    pub id: Uuid,
    pub name: String,
    pub word: String,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
    pub board_id: Option<Uuid>,
}

#[async_trait::async_trait]
impl AdminBbsRepository for AdminBbsRepositoryImpl {
    async fn get_boards_by_key(&self, keys: Option<Vec<String>>) -> anyhow::Result<Vec<Board>> {
        let pool = &self.0;
        let key_lists = if let Some(keys) = &keys {
            let mut initial = "WHERE board_key IN (".to_string();
            initial.push_str(&keys.iter().map(|_| "?").collect::<Vec<_>>().join(", "));
            initial.push(')');
            initial
        } else {
            "".to_string()
        };

        let query = format!(
            r#"
            SELECT
                id,
                name,
                board_key,
                default_name,
                (
                    SELECT
                        COUNT(*)
                    FROM
                        threads
                    WHERE
                        board_id = boards.id
                ) AS thread_count
            FROM
                boards
            {key_lists}
            "#
        );

        let mut query = sqlx::query_as::<_, SelectionBoardWithThreadCount>(&query);

        if let Some(keys) = keys {
            for key in keys {
                query = query.bind(key);
            }
        }

        let selected_boards = query.fetch_all(pool).await?;

        Ok(selected_boards
            .into_iter()
            .map(|board| Board {
                id: Uuid::from_slice(&board.id).unwrap(),
                name: board.name,
                board_key: board.board_key,
                default_name: board.default_name,
                thread_count: board.thread_count,
            })
            .collect())
    }

    async fn create_board(&self, board: CreateBoardInput) -> anyhow::Result<Board> {
        let pool = &self.0;
        let board_id = Uuid::now_v7();

        let mut sets = Vec::new();
        let mut values = Vec::new();

        if let Some(base_thread_creation_span_sec) = board.base_thread_creation_span_sec {
            sets.push("base_thread_creation_span_sec");
            values.push(base_thread_creation_span_sec);
        }
        if let Some(base_response_creation_span_sec) = board.base_response_creation_span_sec {
            sets.push("base_response_creation_span_sec");
            values.push(base_response_creation_span_sec);
        }
        if let Some(max_thread_name_byte_length) = board.max_thread_name_byte_length {
            sets.push("max_thread_name_byte_length");
            values.push(max_thread_name_byte_length);
        }
        if let Some(max_author_name_byte_length) = board.max_author_name_byte_length {
            sets.push("max_author_name_byte_length");
            values.push(max_author_name_byte_length);
        }
        if let Some(max_email_byte_length) = board.max_email_byte_length {
            sets.push("max_email_byte_length");
            values.push(max_email_byte_length);
        }
        if let Some(max_response_body_byte_length) = board.max_response_body_byte_length {
            sets.push("max_response_body_byte_length");
            values.push(max_response_body_byte_length);
        }
        if let Some(max_response_body_lines) = board.max_response_body_lines {
            sets.push("max_response_body_lines");
            values.push(max_response_body_lines);
        }
        if let Some(threads_archive_trigger_thread_count) =
            board.threads_archive_trigger_thread_count
        {
            sets.push("threads_archive_trigger_thread_count");
            values.push(threads_archive_trigger_thread_count);
        }
        if board.threads_archive_cron.is_some() {
            sets.push("threads_archive_cron");
        }

        let mut tx = pool.begin().await?;

        query!(
            r#"
            INSERT INTO
                boards (id, name, board_key, default_name)
            VALUES
                (?, ?, ?, ?)
        "#,
            board_id,
            board.name,
            board.board_key,
            board.default_name
        )
        .execute(&mut *tx)
        .await?;

        let query = format!(
            r#"
            INSERT INTO
                boards_info (id, created_at, updated_at, local_rules, {})
            VALUES
                (?, NOW(), NOW(), ?, {})
            "#,
            sets.join(", "),
            sets.iter().map(|_| "?").collect::<Vec<_>>().join(", ")
        );
        let mut query = sqlx::query(&query).bind(board_id).bind(board.local_rule);

        for v in values {
            query = query.bind(v as i32);
        }
        if let Some(threads_archive_cron) = &board.threads_archive_cron {
            query = query.bind(threads_archive_cron);
        }

        query.execute(&mut *tx).await?;

        tx.commit().await?;

        self.get_boards_by_key(Some(vec![board.board_key.clone()]))
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Failed to create board"))
    }

    async fn get_threads_by_thread_id(
        &self,
        board_key: &str,
        thread_numbers: Option<Vec<u64>>,
    ) -> anyhow::Result<Vec<Thread>> {
        let pool = &self.0;

        let thread_numbers_where = if let Some(thread_numbers) = &thread_numbers {
            let mut initial = "AND thread_number IN (".to_string();
            initial.push_str(
                &thread_numbers
                    .iter()
                    .map(|_| "?")
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            initial.push(')');
            initial
        } else {
            "".to_string()
        };

        let query = format!(
            r#"
            SELECT
                *
            FROM
                threads
            WHERE
                board_id = (
                    SELECT
                        id
                    FROM
                        boards
                    WHERE
                        board_key = ?
                )
            {thread_numbers_where}
            "#
        );

        let mut query = sqlx::query_as::<_, SelectionThread>(&query);

        query = query.bind(board_key);
        if let Some(thread_numbers) = &thread_numbers {
            for thread_number in thread_numbers {
                query = query.bind(thread_number);
            }
        }

        let selected_threads = query.fetch_all(pool).await?;

        Ok(selected_threads
            .into_iter()
            .map(|thread| Thread {
                id: Uuid::from_slice(&thread.id).unwrap(),
                board_id: Uuid::from_slice(&thread.board_id).unwrap(),
                thread_number: thread.thread_number as u64,
                last_modified: thread.last_modified_at,
                sage_last_modified: thread.sage_last_modified_at,
                title: thread.title,
                authed_token_id: Uuid::from_slice(&thread.authed_token_id).unwrap(),
                metadent: thread.metadent,
                response_count: thread.response_count as u32,
                no_pool: thread.no_pool,
                archived: thread.archived,
                active: thread.active,
            })
            .collect())
    }

    async fn get_archived_threads_by_thread_id(
        &self,
        board_key: &str,
        thread_numbers: Option<Vec<u64>>,
    ) -> anyhow::Result<Vec<Thread>> {
        let pool = &self.0;

        let thread_numbers_where = if let Some(thread_numbers) = &thread_numbers {
            let mut initial = "AND thread_number IN (".to_string();
            initial.push_str(
                &thread_numbers
                    .iter()
                    .map(|_| "?")
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            initial.push(')');
            initial
        } else {
            "".to_string()
        };

        let query = format!(
            r#"
            SELECT
                *
            FROM
                archived_threads
            WHERE
                board_id = (
                    SELECT
                        id
                    FROM
                        boards
                    WHERE
                        board_key = ?
                )
            {thread_numbers_where}
            "#
        );

        let mut query = sqlx::query_as::<_, SelectionThread>(&query);

        query = query.bind(board_key);
        if let Some(thread_numbers) = &thread_numbers {
            for thread_number in thread_numbers {
                query = query.bind(thread_number);
            }
        }

        let selected_threads = query.fetch_all(pool).await?;

        Ok(selected_threads
            .into_iter()
            .map(|thread| Thread {
                id: Uuid::from_slice(&thread.id).unwrap(),
                board_id: Uuid::from_slice(&thread.board_id).unwrap(),
                thread_number: thread.thread_number as u64,
                last_modified: thread.last_modified_at,
                sage_last_modified: thread.sage_last_modified_at,
                title: thread.title,
                authed_token_id: Uuid::from_slice(&thread.authed_token_id).unwrap(),
                metadent: thread.metadent,
                response_count: thread.response_count as u32,
                no_pool: thread.no_pool,
                archived: thread.archived,
                active: thread.active,
            })
            .collect())
    }

    async fn get_reses_by_thread_id(
        &self,
        board_key: &str,
        thread_number: u64,
    ) -> anyhow::Result<Vec<Res>> {
        let pool = &self.0;

        let query = query_as!(
            SelectionRes,
            r#"
            SELECT
                id,
                author_name,
                mail,
                body,
                created_at,
                author_id,
                ip_addr,
                authed_token_id,
                board_id,
                thread_id,
                is_abone,
                client_info AS "client_info!: Json<ClientInfo>",
                res_order
            FROM
                responses
            WHERE
                thread_id = (
                    SELECT
                        id
                    FROM
                        threads
                    WHERE
                        board_id = (
                        SELECT
                            id
                        FROM
                            boards
                        WHERE
                            board_key = ?
                        )
                    AND
                        thread_number = ?
                )
            ORDER BY
                res_order ASC
            "#,
            board_key,
            thread_number
        );

        let selected_reses = query.fetch_all(pool).await?;

        Ok(selected_reses
            .into_iter()
            .map(|res| Res {
                id: Uuid::from_slice(&res.id).unwrap(),
                author_name: Some(res.author_name),
                mail: Some(res.mail),
                body: res.body,
                created_at: Utc.from_utc_datetime(&res.created_at),
                author_id: res.author_id,
                ip_addr: res.ip_addr,
                authed_token_id: Uuid::from_slice(&res.authed_token_id).unwrap(),
                board_id: Uuid::from_slice(&res.board_id).unwrap(),
                thread_id: Uuid::from_slice(&res.thread_id).unwrap(),
                is_abone: res.is_abone != 0,
                client_info: res.client_info.0.into(),
                res_order: res.res_order,
            })
            .collect())
    }

    async fn get_archived_reses_by_thread_id(
        &self,
        board_key: &str,
        thread_number: u64,
    ) -> anyhow::Result<Vec<Res>> {
        let pool = &self.0;

        let query = query_as!(
            SelectionRes,
            r#"
            SELECT
                id,
                author_name,
                mail,
                body,
                created_at,
                author_id,
                ip_addr,
                authed_token_id,
                board_id,
                thread_id,
                is_abone,
                client_info AS "client_info!: Json<ClientInfo>",
                res_order
            FROM
                archived_responses
            WHERE
                thread_id = (
                    SELECT
                        id
                    FROM
                        archived_threads
                    WHERE
                        board_id = (
                        SELECT
                            id
                        FROM
                            boards
                        WHERE
                            board_key = ?
                        )
                    AND
                        thread_number = ?
                )
            ORDER BY
                res_order ASC
            "#,
            board_key,
            thread_number
        );

        let selected_reses = query.fetch_all(pool).await?;

        Ok(selected_reses
            .into_iter()
            .map(|res| Res {
                id: Uuid::from_slice(&res.id).unwrap(),
                author_name: Some(res.author_name),
                mail: Some(res.mail),
                body: res.body,
                created_at: Utc.from_utc_datetime(&res.created_at),
                author_id: res.author_id,
                ip_addr: res.ip_addr,
                authed_token_id: Uuid::from_slice(&res.authed_token_id).unwrap(),
                board_id: Uuid::from_slice(&res.board_id).unwrap(),
                thread_id: Uuid::from_slice(&res.thread_id).unwrap(),
                is_abone: res.is_abone != 0,
                client_info: res.client_info.0.into(),
                res_order: res.res_order,
            })
            .collect())
    }

    async fn get_archived_threads_by_filter(
        &self,
        board_key: &str,
        keyword: Option<&str>,
        range: (Option<chrono::DateTime<Utc>>, Option<chrono::DateTime<Utc>>),
        page: u64,
        limit: u64, // <= 100
    ) -> anyhow::Result<Vec<Thread>> {
        let pool = &self.0;

        let mut query = r#"
            SELECT
                *
            FROM
                archived_threads
            WHERE
                board_id = (
                    SELECT
                        id
                    FROM
                        boards
                    WHERE
                        board_key = ?
                )
            "#
        .to_string();

        if keyword.is_some() {
            query.push_str("AND title LIKE ? ");
        }

        if matches!(range, (Some(_), Some(_))) {
            query.push_str("AND last_modified_at BETWEEN ? AND ? ");
        }

        query.push_str("ORDER BY last_modified_at DESC ");
        query.push_str("LIMIT ? OFFSET ?");

        let mut query = sqlx::query_as::<_, SelectionThread>(&query);

        query = query.bind(board_key);
        if let Some(keyword) = keyword {
            query = query.bind(format!("%{}%", keyword));
        }
        if let (Some(start), Some(end)) = range {
            query = query.bind(start).bind(end);
        }
        query = query.bind(limit).bind(page * limit);

        let selected_threads = query.fetch_all(pool).await?;

        Ok(selected_threads
            .into_iter()
            .map(|thread| Thread {
                id: Uuid::from_slice(&thread.id).unwrap(),
                board_id: Uuid::from_slice(&thread.board_id).unwrap(),
                thread_number: thread.thread_number as u64,
                last_modified: thread.last_modified_at,
                sage_last_modified: thread.sage_last_modified_at,
                title: thread.title,
                authed_token_id: Uuid::from_slice(&thread.authed_token_id).unwrap(),
                metadent: thread.metadent,
                response_count: thread.response_count as u32,
                no_pool: thread.no_pool,
                archived: thread.archived,
                active: thread.active,
            })
            .collect())
    }

    async fn get_res(
        &self,
        res_id: Uuid,
    ) -> anyhow::Result<(Res, String, String, u64, Option<String>)> {
        let pool = &self.0;

        let res = query_as!(
            SelectionRes,
            r#"
            SELECT
                id,
                author_name,
                mail,
                body,
                created_at,
                author_id,
                ip_addr,
                authed_token_id,
                board_id,
                thread_id,
                is_abone,
                client_info AS "client_info!: Json<ClientInfo>",
                res_order
            FROM
                responses
            WHERE
                id = ?
            "#,
            res_id.as_bytes().to_vec()
        )
        .fetch_one(pool)
        .await?;

        let res = Res {
            id: Uuid::from_slice(&res.id).unwrap(),
            author_name: Some(res.author_name),
            mail: Some(res.mail),
            body: res.body,
            created_at: Utc.from_utc_datetime(&res.created_at),
            author_id: res.author_id,
            ip_addr: res.ip_addr,
            authed_token_id: Uuid::from_slice(&res.authed_token_id).unwrap(),
            board_id: Uuid::from_slice(&res.board_id).unwrap(),
            thread_id: Uuid::from_slice(&res.thread_id).unwrap(),
            is_abone: res.is_abone != 0,
            client_info: res.client_info.0.into(),
            res_order: res.res_order,
        };

        struct BoardKeyThreadNumber {
            board_key: String,
            thread_number: u64,
            default_name: String,
            thread_title: Option<String>,
        }

        let board_key = query_as!(
            BoardKeyThreadNumber,
            r#"
            SELECT
                boards.board_key AS "board_key!: String",
                threads.thread_number AS "thread_number!: u64",
                boards.default_name AS "default_name!: String",
                threads.title AS "thread_title: String"
            FROM
                boards
            JOIN threads ON boards.id = threads.board_id
            WHERE
                threads.id = ?
            "#,
            res.thread_id,
        )
        .fetch_one(pool)
        .await?;

        Ok((
            res,
            board_key.default_name,
            board_key.board_key,
            board_key.thread_number,
            board_key.thread_title,
        ))
    }

    async fn update_res(
        &self,
        id: Uuid,
        author_name: Option<String>,
        mail: Option<String>,
        body: Option<String>,
        is_abone: Option<bool>,
    ) -> anyhow::Result<Res> {
        let pool = &self.0;

        let mut sets = Vec::new();
        let mut values = Vec::new();

        if let Some(author_name) = author_name {
            sets.push("author_name = ?");
            values.push(author_name);
        }
        if let Some(mail) = mail {
            sets.push("mail = ?");
            values.push(mail);
        }
        if let Some(body) = body {
            sets.push("body = ?");
            values.push(body);
        }
        if is_abone.is_some() {
            sets.push("is_abone = ?");
        }

        let query = format!(
            r#"
            UPDATE
                responses
            SET
                {}
            WHERE
                id = ?
            "#,
            sets.join(", ")
        );

        let mut query = sqlx::query(&query);
        for v in values {
            query = query.bind(v);
        }
        if let Some(is_abone) = is_abone {
            query = query.bind(is_abone);
        }
        let query = query.bind(id.as_bytes().to_vec());

        query.execute(pool).await?;

        let res = query_as!(
            SelectionRes,
            r#"
            SELECT
                id,
                author_name,
                mail,
                body,
                created_at,
                author_id,
                ip_addr,
                authed_token_id,
                board_id,
                thread_id,
                is_abone,
                client_info AS "client_info!: Json<ClientInfo>",
                res_order
            FROM
                responses
            WHERE
                id = ?
            "#,
            id.as_bytes().to_vec()
        )
        .fetch_one(pool)
        .await?;

        Ok(Res {
            id: Uuid::from_slice(&res.id).unwrap(),
            author_name: Some(res.author_name),
            mail: Some(res.mail),
            body: res.body,
            created_at: Utc.from_utc_datetime(&res.created_at),
            author_id: res.author_id,
            ip_addr: res.ip_addr,
            authed_token_id: Uuid::from_slice(&res.authed_token_id).unwrap(),
            board_id: Uuid::from_slice(&res.board_id).unwrap(),
            thread_id: Uuid::from_slice(&res.thread_id).unwrap(),
            is_abone: res.is_abone != 0,
            client_info: res.client_info.0.into(),
            res_order: res.res_order,
        })
    }

    async fn get_authed_token(&self, id: Uuid) -> anyhow::Result<AuthedToken> {
        let query = query_as!(
            AuthedToken,
            r#"
            SELECT
                id AS "id!: Uuid",
                token,
                origin_ip,
                reduced_origin_ip,
                writing_ua,
                authed_ua,
                created_at,
                authed_at,
                validity AS "validity!: bool",
                last_wrote_at
            FROM
                authed_tokens
            WHERE
                id = ?
            "#,
            id.as_bytes().to_vec(),
        );

        let authed_token = query.fetch_one(&self.0).await?;

        Ok(authed_token)
    }

    async fn delete_authed_token(&self, id: Uuid) -> anyhow::Result<()> {
        let query = query!(
            r#"
            UPDATE
                authed_tokens
            SET
                validity = 0
            WHERE
                id = ?
        "#,
            id.as_bytes().to_vec(),
        );

        query.execute(&self.0).await?;

        Ok(())
    }

    async fn delete_authed_token_by_origin_ip(&self, id: Uuid) -> anyhow::Result<()> {
        let query = query!(
            r#"
            UPDATE
                authed_tokens
            SET
                validity = 0
            WHERE
                id IN (
                    SELECT id FROM (
                        SELECT
                            id
                        FROM
                            authed_tokens
                        WHERE
                            origin_ip = ?
                    ) tmp      
                )
        "#,
            id.as_bytes().to_vec(),
        );

        query.execute(&self.0).await?;

        Ok(())
    }

    async fn get_ng_words(&self) -> anyhow::Result<Vec<NgWord>> {
        let selections = query_as!(
            SelectionNgWord,
            r#"
            SELECT
                ng.id AS "id!: Uuid",
                name AS "name!: String",
                word AS "word!: String",
                created_at AS "created_at!: chrono::DateTime<Utc>",
                updated_at AS "updated_at!: chrono::DateTime<Utc>",
                board_id AS "board_id: Uuid"
            FROM
                ng_words AS ng
                LEFT OUTER JOIN boards_ng_words AS bng
                ON ng.id = bng.ng_word_id
            "#,
        )
        .fetch_all(&self.0)
        .await?;

        let mut ng_words_map = HashMap::<_, NgWord>::new();
        for selection in selections {
            ng_words_map
                .entry(selection.id)
                .and_modify(|x| {
                    if let Some(board_id) = selection.board_id {
                        x.board_ids.push(board_id);
                    }
                })
                .or_insert(NgWord {
                    id: selection.id,
                    name: selection.name,
                    word: selection.word,
                    created_at: selection.created_at,
                    updated_at: selection.updated_at,
                    board_ids: if let Some(board_id) = selection.board_id {
                        vec![board_id]
                    } else {
                        Vec::new()
                    },
                });
        }

        Ok(ng_words_map.into_values().collect())
    }

    async fn create_ng_word(&self, name: &str, word: &str) -> anyhow::Result<NgWord> {
        let id = Uuid::now_v7();

        let query = query!(
            r#"
            INSERT INTO
                ng_words (id, name, word, created_at, updated_at)
            VALUES
                (?, ?, ?, NOW(), NOW())
        "#,
            id,
            name,
            word
        );
        self.0.execute(query).await?;

        let query = query_as!(
            SelectionNgWord,
            r#"
            SELECT
                ng.id AS "id!: Uuid",
                name AS "name!: String",
                word AS "word!: String",
                created_at AS "created_at!: chrono::DateTime<Utc>",
                updated_at AS "updated_at!: chrono::DateTime<Utc>",
                board_id AS "board_id: Uuid"
            FROM
                ng_words AS ng
                LEFT OUTER JOIN boards_ng_words AS bng
                ON ng.id = bng.ng_word_id
            WHERE
                ng.id = ?
            "#,
            id,
        );

        let selection = query.fetch_one(&self.0).await?;

        Ok(NgWord {
            id: selection.id,
            name: selection.name,
            word: selection.word,
            created_at: selection.created_at,
            updated_at: selection.updated_at,
            board_ids: if let Some(board_id) = selection.board_id {
                vec![board_id]
            } else {
                Vec::new()
            },
        })
    }

    async fn delete_ng_word(&self, ng_word_id: Uuid) -> anyhow::Result<()> {
        let query = query!(
            r#"
            DELETE FROM
                ng_words
            WHERE
                id = ?
        "#,
            ng_word_id
        );

        self.0.execute(query).await?;

        Ok(())
    }

    async fn update_ng_word(
        &self,
        id: Uuid,
        name: Option<&str>,
        word: Option<&str>,
        board_ids: Option<Vec<Uuid>>,
    ) -> anyhow::Result<NgWord> {
        let mut sets = Vec::new();
        let mut values = Vec::new();

        if let Some(name) = name {
            sets.push("name = ?");
            values.push(name);
        }
        if let Some(word) = word {
            sets.push("word = ?");
            values.push(word);
        }

        let query = format!(
            r#"
            UPDATE
                ng_words
            SET
                {}
            WHERE
                id = ?
            "#,
            sets.join(", ")
        );

        let mut query = sqlx::query(&query);
        for v in values {
            query = query.bind(v);
        }
        let query = query.bind(id);
        query.execute(&self.0).await?;

        if let Some(board_ids) = board_ids {
            let mut tx = self.0.begin().await?;

            let query = query!(
                r#"
                DELETE FROM
                    boards_ng_words
                WHERE
                    ng_word_id = ?
            "#,
                id
            );
            tx.execute(query).await?;

            for board_id in board_ids {
                let bnw_id = Uuid::now_v7();
                let query = query!(
                    r#"
                    INSERT INTO
                        boards_ng_words (id, board_id, ng_word_id)
                    VALUES
                        (?, ?, ?)
                "#,
                    bnw_id,
                    board_id,
                    id
                );
                tx.execute(query).await?;
            }

            tx.commit().await?;
        }

        let query = query_as!(
            SelectionNgWord,
            r#"
            SELECT
                ng.id AS "id!: Uuid",
                name AS "name!: String",
                word AS "word!: String",
                created_at AS "created_at!: chrono::DateTime<Utc>",
                updated_at AS "updated_at!: chrono::DateTime<Utc>",
                board_id AS "board_id: Uuid"
            FROM
                ng_words AS ng
                LEFT OUTER JOIN boards_ng_words AS bng
                ON ng.id = bng.ng_word_id
            WHERE
                ng.id = ?
            "#,
            id,
        );

        let selections = query.fetch_all(&self.0).await?;
        let board_ids = selections
            .iter()
            .filter_map(|selection| selection.board_id)
            .collect::<Vec<_>>();
        let selection = selections.into_iter().next().unwrap();

        Ok(NgWord {
            id: selection.id,
            name: selection.name,
            word: selection.word,
            created_at: selection.created_at,
            updated_at: selection.updated_at,
            board_ids,
        })
    }
}
