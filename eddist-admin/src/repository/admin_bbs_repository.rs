use std::vec;

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use eddist_core::domain::client_info::ClientInfo;
use sqlx::{query, query_as, types::Json, FromRow, MySqlPool};
use uuid::Uuid;

use crate::{Board, BoardInfo, CreateBoardInput, EditBoardInput, Res, Thread};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RestrictionType {
    CreatingResponse,
    CreatingThread,
    AuthCode,
    All,
}

impl RestrictionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RestrictionType::CreatingResponse => "creating_response",
            RestrictionType::CreatingThread => "creating_thread",
            RestrictionType::AuthCode => "auth_code",
            RestrictionType::All => "all",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "creating_response" => Some(RestrictionType::CreatingResponse),
            "creating_thread" => Some(RestrictionType::CreatingThread),
            "auth_code" => Some(RestrictionType::AuthCode),
            "all" => Some(RestrictionType::All),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UserRestrictionRule {
    pub id: Uuid,
    pub name: String,
    pub filter_expression: String,
    pub restriction_type: RestrictionType,
    pub active: bool,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
    pub created_by: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug)]
pub struct UpdateUserRestrictionRule {
    pub name: Option<String>,
    pub filter_expression: Option<String>,
    pub restriction_type: Option<RestrictionType>,
    pub active: Option<bool>,
    pub description: Option<String>,
}

#[async_trait::async_trait]
pub trait AdminBbsRepository: Send + Sync {
    async fn get_boards_by_key(&self, keys: Option<Vec<String>>) -> anyhow::Result<Vec<Board>>;
    async fn get_board_info(&self, id: Uuid) -> anyhow::Result<BoardInfo>;

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
    async fn edit_board(&self, board_key: &str, board: EditBoardInput) -> anyhow::Result<Board>;

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

    async fn compact_threads(&self, board_key: &str, target_count: u32) -> anyhow::Result<()>;

    // User restriction rules
    async fn get_all_user_restriction_rules(&self) -> anyhow::Result<Vec<UserRestrictionRule>>;
    async fn get_user_restriction_rule(
        &self,
        id: Uuid,
    ) -> anyhow::Result<Option<UserRestrictionRule>>;
    async fn create_user_restriction_rule(&self, rule: &UserRestrictionRule) -> anyhow::Result<()>;
    async fn update_user_restriction_rule(
        &self,
        id: Uuid,
        update: &UpdateUserRestrictionRule,
    ) -> anyhow::Result<Option<UserRestrictionRule>>;
    async fn delete_user_restriction_rule(&self, id: Uuid) -> anyhow::Result<bool>;
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
pub struct SelectionBoardInfo {
    pub local_rules: String,
    pub base_thread_creation_span_sec: i32,
    pub base_response_creation_span_sec: i32,
    pub max_thread_name_byte_length: i32,
    pub max_author_name_byte_length: i32,
    pub max_email_byte_length: i32,
    pub max_response_body_byte_length: i32,
    pub max_response_body_lines: i32,
    pub threads_archive_trigger_thread_count: Option<i32>,
    pub threads_archive_cron: Option<String>,
    pub read_only: bool,
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

    async fn get_board_info(&self, id: Uuid) -> anyhow::Result<BoardInfo> {
        let pool = &self.0;

        let board = query_as!(
            SelectionBoardInfo,
            r#"
            SELECT
                local_rules,
                base_thread_creation_span_sec,
                base_response_creation_span_sec,
                max_thread_name_byte_length,
                max_author_name_byte_length,
                max_email_byte_length,
                max_response_body_byte_length,
                max_response_body_lines,
                threads_archive_trigger_thread_count,
                threads_archive_cron,
                read_only AS "read_only!: bool"
            FROM
                boards_info
            WHERE
                id = ?
            "#,
            id.as_bytes().to_vec().to_vec()
        )
        .fetch_one(pool)
        .await?;

        Ok(BoardInfo {
            local_rules: board.local_rules,
            base_thread_creation_span_sec: board.base_thread_creation_span_sec as usize,
            base_response_creation_span_sec: board.base_response_creation_span_sec as usize,
            max_thread_name_byte_length: board.max_thread_name_byte_length as usize,
            max_author_name_byte_length: board.max_author_name_byte_length as usize,
            max_email_byte_length: board.max_email_byte_length as usize,
            max_response_body_byte_length: board.max_response_body_byte_length as usize,
            max_response_body_lines: board.max_response_body_lines as usize,
            threads_archive_trigger_thread_count: board
                .threads_archive_trigger_thread_count
                .map(|v| v as usize),
            threads_archive_cron: board.threads_archive_cron,
            read_only: board.read_only,
        })
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

    async fn edit_board(&self, board_key: &str, board: EditBoardInput) -> anyhow::Result<Board> {
        let pool = &self.0;

        let mut sets = Vec::new();
        let mut values = Vec::new();
        let mut values_str = Vec::new();

        let mut board_sets = Vec::new();
        let mut board_values = Vec::new();

        // for boards
        if let Some(name) = &board.name {
            board_sets.push("name = ?");
            board_values.push(name);
        }
        if let Some(default_name) = &board.default_name {
            board_sets.push("default_name = ?");
            board_values.push(default_name);
        }

        // str first for board_info
        if let Some(local_rule) = &board.local_rule {
            sets.push("local_rules = ?");
            values_str.push(local_rule);
        }
        if let Some(threads_archive_cron) = &board.threads_archive_cron {
            sets.push("threads_archive_cron = ?");
            values_str.push(threads_archive_cron);
        }

        if let Some(base_thread_creation_span_sec) = board.base_thread_creation_span_sec {
            sets.push("base_thread_creation_span_sec = ?");
            values.push(base_thread_creation_span_sec);
        }
        if let Some(base_response_creation_span_sec) = board.base_response_creation_span_sec {
            sets.push("base_response_creation_span_sec = ?");
            values.push(base_response_creation_span_sec);
        }
        if let Some(max_thread_name_byte_length) = board.max_thread_name_byte_length {
            sets.push("max_thread_name_byte_length = ?");
            values.push(max_thread_name_byte_length);
        }
        if let Some(max_author_name_byte_length) = board.max_author_name_byte_length {
            sets.push("max_author_name_byte_length = ?");
            values.push(max_author_name_byte_length);
        }
        if let Some(max_email_byte_length) = board.max_email_byte_length {
            sets.push("max_email_byte_length = ?");
            values.push(max_email_byte_length);
        }
        if let Some(max_response_body_byte_length) = board.max_response_body_byte_length {
            sets.push("max_response_body_byte_length = ?");
            values.push(max_response_body_byte_length);
        }
        if let Some(max_response_body_lines) = board.max_response_body_lines {
            sets.push("max_response_body_lines = ?");
            values.push(max_response_body_lines);
        }
        if let Some(threads_archive_trigger_thread_count) =
            board.threads_archive_trigger_thread_count
        {
            sets.push("threads_archive_trigger_thread_count = ?");
            values.push(threads_archive_trigger_thread_count);
        }
        if board.read_only.is_some() {
            sets.push("read_only = ?");
        }

        let mut tx = pool.begin().await?;

        if !sets.is_empty() {
            let query = format!(
                r#"
            UPDATE
                boards_info
            SET
                {}
            WHERE
                id = (
                    SELECT
                        id
                    FROM
                        boards
                    WHERE
                        board_key = ?
                )
            "#,
                sets.join(", ")
            );
            let mut query = sqlx::query(&query);

            for v in values_str {
                query = query.bind(v);
            }
            for v in values {
                query = query.bind(v as i32);
            }
            if let Some(read_only) = board.read_only {
                query = query.bind(read_only);
            }
            let query = query.bind(board_key);

            query.execute(&mut *tx).await?;
        }

        if !board_sets.is_empty() {
            let query = format!(
                r#"
            UPDATE
                boards
            SET
                {}
            WHERE
                board_key = ?
            "#,
                board_sets.join(", ")
            );
            let mut query = sqlx::query(&query);

            for v in board_values {
                query = query.bind(v);
            }
            let query = query.bind(board_key);

            query.execute(&mut *tx).await?;
        }

        tx.commit().await?;

        self.get_boards_by_key(Some(vec![board_key.to_string()]))
            .await?
            .first()
            .cloned()
            .ok_or(anyhow::anyhow!("Failed to edit board"))
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
            id.as_bytes().to_vec().to_vec()
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

    async fn compact_threads(&self, board_key: &str, target_count: u32) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
            UPDATE threads SET archived = 1, active = 0 WHERE id IN (
                SELECT id FROM (
                    SELECT id
                    FROM threads
                    WHERE board_id = (SELECT id FROM boards WHERE board_key = ?)
                    AND archived = 0
                    ORDER BY last_modified_at DESC
                    LIMIT 1000000 OFFSET ?
                ) AS tmp
            )
            "#,
            board_key,
            target_count,
        )
        .execute(&self.0)
        .await?;

        Ok(())
    }

    async fn get_all_user_restriction_rules(&self) -> anyhow::Result<Vec<UserRestrictionRule>> {
        let rows = query_as!(
            UserRestrictionRuleRow,
            r#"
            SELECT id, name, filter_expression, restriction_type, active, created_at, updated_at, created_by, description
            FROM user_restriction_rules
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&self.0)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn get_user_restriction_rule(
        &self,
        id: Uuid,
    ) -> anyhow::Result<Option<UserRestrictionRule>> {
        let row = query_as!(
            UserRestrictionRuleRow,
            r#"
            SELECT id, name, filter_expression, restriction_type, active, created_at, updated_at, created_by, description
            FROM user_restriction_rules
            WHERE id = ?
            "#,
            id.as_bytes().to_vec()
        )
        .fetch_optional(&self.0)
        .await?;

        Ok(row.map(Into::into))
    }

    async fn create_user_restriction_rule(&self, rule: &UserRestrictionRule) -> anyhow::Result<()> {
        query!(
            r#"
            INSERT INTO user_restriction_rules (id, name, filter_expression, restriction_type, active, created_at, updated_at, created_by, description)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            rule.id.as_bytes().to_vec(),
            rule.name,
            rule.filter_expression,
            rule.restriction_type.as_str(),
            rule.active,
            rule.created_at,
            rule.updated_at,
            rule.created_by,
            rule.description
        )
        .execute(&self.0)
        .await?;

        Ok(())
    }

    async fn update_user_restriction_rule(
        &self,
        id: Uuid,
        update: &UpdateUserRestrictionRule,
    ) -> anyhow::Result<Option<UserRestrictionRule>> {
        let mut set_clauses = vec!["updated_at = CURRENT_TIMESTAMP".to_string()];
        let mut values: Vec<String> = vec![];

        if let Some(ref name) = update.name {
            set_clauses.push("name = ?".to_string());
            values.push(name.clone());
        }
        if let Some(ref filter_expression) = update.filter_expression {
            set_clauses.push("filter_expression = ?".to_string());
            values.push(filter_expression.clone());
        }
        if let Some(ref restriction_type) = update.restriction_type {
            set_clauses.push("restriction_type = ?".to_string());
            values.push(restriction_type.as_str().to_string());
        }
        if let Some(active) = update.active {
            set_clauses.push("active = ?".to_string());
            values.push(active.to_string());
        }
        if let Some(ref description) = update.description {
            set_clauses.push("description = ?".to_string());
            values.push(description.clone());
        }

        let sql = format!(
            "UPDATE user_restriction_rules SET {} WHERE id = ?",
            set_clauses.join(", ")
        );

        let result = query(&sql).execute(&self.0).await?;

        if result.rows_affected() > 0 {
            self.get_user_restriction_rule(id).await
        } else {
            Ok(None)
        }
    }

    async fn delete_user_restriction_rule(&self, id: Uuid) -> anyhow::Result<bool> {
        let result = query!(
            "DELETE FROM user_restriction_rules WHERE id = ?",
            id.as_bytes().to_vec()
        )
        .execute(&self.0)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}

#[derive(FromRow)]
struct UserRestrictionRuleRow {
    id: Vec<u8>,
    name: String,
    filter_expression: String,
    restriction_type: String,
    active: i8,
    created_at: chrono::NaiveDateTime,
    updated_at: chrono::NaiveDateTime,
    created_by: Option<String>,
    description: Option<String>,
}

impl From<UserRestrictionRuleRow> for UserRestrictionRule {
    fn from(row: UserRestrictionRuleRow) -> Self {
        Self {
            id: Uuid::from_slice(&row.id).unwrap(),
            name: row.name,
            filter_expression: row.filter_expression,
            restriction_type: RestrictionType::from_str(&row.restriction_type)
                .unwrap_or(RestrictionType::CreatingResponse),
            active: row.active != 0,
            created_at: DateTime::from_naive_utc_and_offset(row.created_at, Utc),
            updated_at: DateTime::from_naive_utc_and_offset(row.updated_at, Utc),
            created_by: row.created_by,
            description: row.description,
        }
    }
}
