use sqlx::{query, query_as, MySqlPool};
use uuid::Uuid;

use crate::models::{Board, BoardInfo, CreateBoardInput, EditBoardInput};

use super::admin_bbs_repository::{SelectionBoardInfo, SelectionBoardWithThreadCount};

#[async_trait::async_trait]
pub trait AdminBoardRepository: Send + Sync {
    async fn get_boards_by_key(&self, keys: Option<Vec<String>>) -> anyhow::Result<Vec<Board>>;
    async fn get_board_info(&self, id: Uuid) -> anyhow::Result<BoardInfo>;
    async fn create_board(&self, board: CreateBoardInput) -> anyhow::Result<Board>;
    async fn edit_board(&self, board_key: &str, board: EditBoardInput) -> anyhow::Result<Board>;
}

#[derive(Clone)]
pub struct AdminBoardRepositoryImpl(pub(crate) MySqlPool);

impl AdminBoardRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self(pool)
    }
}

#[async_trait::async_trait]
impl AdminBoardRepository for AdminBoardRepositoryImpl {
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
            id.as_bytes().to_vec()
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
}
