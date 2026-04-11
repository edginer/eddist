#[cfg(not(feature = "backend-postgres"))]
use sqlx::{MySqlPool, query, query_as};
#[cfg(feature = "backend-postgres")]
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{Board, BoardInfo, CreateBoardInput, EditBoardInput};

#[cfg(not(feature = "backend-postgres"))]
use super::admin_bbs_repository::{SelectionBoardInfo, SelectionBoardWithThreadCount};
#[cfg(feature = "backend-postgres")]
use super::admin_bbs_repository::{SelectionBoardInfo, SelectionBoardWithThreadCountPg};

#[async_trait::async_trait]
pub trait AdminBoardRepository: Send + Sync {
    async fn get_boards_by_key(&self, keys: Option<Vec<String>>) -> anyhow::Result<Vec<Board>>;
    async fn get_board_info(&self, id: Uuid) -> anyhow::Result<BoardInfo>;
    async fn create_board(&self, board: CreateBoardInput) -> anyhow::Result<Board>;
    async fn edit_board(&self, board_key: &str, board: EditBoardInput) -> anyhow::Result<Board>;
}

#[cfg(not(feature = "backend-postgres"))]
#[derive(Clone)]
pub struct AdminBoardRepositoryImpl(pub(crate) MySqlPool);

#[cfg(not(feature = "backend-postgres"))]
impl AdminBoardRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self(pool)
    }
}

#[cfg(not(feature = "backend-postgres"))]
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
                read_only AS "read_only!: bool",
                force_metadent_type
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
            force_metadent_type: board.force_metadent_type,
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
        if board.force_metadent_type.is_some() {
            sets.push("force_metadent_type");
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
        if let Some(force_metadent_type) = &board.force_metadent_type {
            query = query.bind(force_metadent_type);
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
        match &board.threads_archive_cron {
            Some(v) if v.is_empty() => {
                sets.push("threads_archive_cron = NULL");
            }
            Some(v) => {
                sets.push("threads_archive_cron = ?");
                values_str.push(v);
            }
            None => {}
        }
        match &board.force_metadent_type {
            Some(v) if v.is_empty() => {
                sets.push("force_metadent_type = NULL");
            }
            Some(v) => {
                sets.push("force_metadent_type = ?");
                values_str.push(v);
            }
            None => {}
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

#[cfg(feature = "backend-postgres")]
#[derive(Clone)]
pub struct AdminBoardRepositoryPgImpl(pub(crate) PgPool);

#[cfg(feature = "backend-postgres")]
impl AdminBoardRepositoryPgImpl {
    pub fn new(pool: PgPool) -> Self {
        Self(pool)
    }
}

#[cfg(feature = "backend-postgres")]
#[async_trait::async_trait]
impl AdminBoardRepository for AdminBoardRepositoryPgImpl {
    async fn get_boards_by_key(&self, keys: Option<Vec<String>>) -> anyhow::Result<Vec<Board>> {
        let pool = &self.0;

        let selected_boards = if let Some(ref keys) = keys {
            sqlx::query_as::<_, SelectionBoardWithThreadCountPg>(
                r#"
                SELECT
                    id,
                    name,
                    board_key,
                    default_name,
                    (SELECT COUNT(*) FROM threads WHERE board_id = boards.id) AS thread_count
                FROM boards
                WHERE board_key = ANY($1)
                "#,
            )
            .bind(keys)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query_as::<_, SelectionBoardWithThreadCountPg>(
                r#"
                SELECT
                    id,
                    name,
                    board_key,
                    default_name,
                    (SELECT COUNT(*) FROM threads WHERE board_id = boards.id) AS thread_count
                FROM boards
                "#,
            )
            .fetch_all(pool)
            .await?
        };

        Ok(selected_boards
            .into_iter()
            .map(|board| Board {
                id: board.id,
                name: board.name,
                board_key: board.board_key,
                default_name: board.default_name,
                thread_count: board.thread_count,
            })
            .collect())
    }

    async fn get_board_info(&self, id: Uuid) -> anyhow::Result<BoardInfo> {
        let pool = &self.0;

        let board = sqlx::query_as::<_, SelectionBoardInfo>(
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
                read_only,
                force_metadent_type
            FROM boards_info
            WHERE id = $1
            "#,
        )
        .bind(id)
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
            force_metadent_type: board.force_metadent_type,
        })
    }

    async fn create_board(&self, board: CreateBoardInput) -> anyhow::Result<Board> {
        let pool = &self.0;
        let board_id = Uuid::now_v7();

        let mut tx = pool.begin().await?;

        sqlx::query(
            "INSERT INTO boards (id, name, board_key, default_name) VALUES ($1, $2, $3, $4)",
        )
        .bind(board_id)
        .bind(&board.name)
        .bind(&board.board_key)
        .bind(&board.default_name)
        .execute(&mut *tx)
        .await?;

        // Build dynamic INSERT for boards_info
        let mut columns = vec!["id", "created_at", "updated_at", "local_rules"];
        let mut int_cols: Vec<&str> = Vec::new();
        let mut int_vals: Vec<i32> = Vec::new();
        let mut str_cols: Vec<&str> = Vec::new();

        if board.base_thread_creation_span_sec.is_some() {
            int_cols.push("base_thread_creation_span_sec");
        }
        if board.base_response_creation_span_sec.is_some() {
            int_cols.push("base_response_creation_span_sec");
        }
        if board.max_thread_name_byte_length.is_some() {
            int_cols.push("max_thread_name_byte_length");
        }
        if board.max_author_name_byte_length.is_some() {
            int_cols.push("max_author_name_byte_length");
        }
        if board.max_email_byte_length.is_some() {
            int_cols.push("max_email_byte_length");
        }
        if board.max_response_body_byte_length.is_some() {
            int_cols.push("max_response_body_byte_length");
        }
        if board.max_response_body_lines.is_some() {
            int_cols.push("max_response_body_lines");
        }
        if board.threads_archive_trigger_thread_count.is_some() {
            int_cols.push("threads_archive_trigger_thread_count");
        }
        if board.threads_archive_cron.is_some() {
            str_cols.push("threads_archive_cron");
        }
        if board.force_metadent_type.is_some() {
            str_cols.push("force_metadent_type");
        }

        // Collect int values after we know which columns are present
        if let Some(v) = board.base_thread_creation_span_sec { int_vals.push(v as i32); }
        if let Some(v) = board.base_response_creation_span_sec { int_vals.push(v as i32); }
        if let Some(v) = board.max_thread_name_byte_length { int_vals.push(v as i32); }
        if let Some(v) = board.max_author_name_byte_length { int_vals.push(v as i32); }
        if let Some(v) = board.max_email_byte_length { int_vals.push(v as i32); }
        if let Some(v) = board.max_response_body_byte_length { int_vals.push(v as i32); }
        if let Some(v) = board.max_response_body_lines { int_vals.push(v as i32); }
        if let Some(v) = board.threads_archive_trigger_thread_count { int_vals.push(v as i32); }

        columns.extend(int_cols.iter());
        columns.extend(str_cols.iter());

        // $1=id, $2=created_at, $3=updated_at, $4=local_rules, then int cols, then str cols
        let now = chrono::Utc::now();
        let mut param_idx = 5usize; // start after the 4 fixed params
        let mut dynamic_placeholders: Vec<String> = Vec::new();
        for _ in &int_cols {
            dynamic_placeholders.push(format!("${param_idx}"));
            param_idx += 1;
        }
        for _ in &str_cols {
            dynamic_placeholders.push(format!("${param_idx}"));
            param_idx += 1;
        }

        let all_placeholders = ["$1", "$2", "$3", "$4"]
            .iter()
            .map(|s| s.to_string())
            .chain(dynamic_placeholders)
            .collect::<Vec<_>>()
            .join(", ");

        let sql = format!(
            "INSERT INTO boards_info ({}) VALUES ({})",
            columns.join(", "),
            all_placeholders
        );

        let mut q = sqlx::query(&sql)
            .bind(board_id)
            .bind(now)
            .bind(now)
            .bind(&board.local_rule);

        for v in &int_vals {
            q = q.bind(v);
        }
        if let Some(ref v) = board.threads_archive_cron {
            q = q.bind(v);
        }
        if let Some(ref v) = board.force_metadent_type {
            q = q.bind(v);
        }

        q.execute(&mut *tx).await?;
        tx.commit().await?;

        self.get_boards_by_key(Some(vec![board.board_key.clone()]))
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Failed to create board"))
    }

    async fn edit_board(&self, board_key: &str, board: EditBoardInput) -> anyhow::Result<Board> {
        let pool = &self.0;

        let mut info_sets: Vec<String> = Vec::new();
        let mut info_str_vals: Vec<String> = Vec::new();
        let mut info_int_vals: Vec<i32> = Vec::new();
        let mut info_bool_val: Option<bool> = None;

        let mut board_sets: Vec<String> = Vec::new();
        let mut board_str_vals: Vec<String> = Vec::new();

        // boards table columns
        if let Some(ref name) = board.name {
            board_sets.push(format!("name = ${}", board_sets.len() + 1));
            board_str_vals.push(name.clone());
        }
        if let Some(ref default_name) = board.default_name {
            board_sets.push(format!("default_name = ${}", board_sets.len() + 1));
            board_str_vals.push(default_name.clone());
        }

        // boards_info columns — build param index tracking
        let mut idx = 1usize;
        if let Some(ref local_rule) = board.local_rule {
            info_sets.push(format!("local_rules = ${idx}"));
            info_str_vals.push(local_rule.clone());
            idx += 1;
        }
        match &board.threads_archive_cron {
            Some(v) if v.is_empty() => {
                info_sets.push("threads_archive_cron = NULL".to_string());
            }
            Some(v) => {
                info_sets.push(format!("threads_archive_cron = ${idx}"));
                info_str_vals.push(v.clone());
                idx += 1;
            }
            None => {}
        }
        match &board.force_metadent_type {
            Some(v) if v.is_empty() => {
                info_sets.push("force_metadent_type = NULL".to_string());
            }
            Some(v) => {
                info_sets.push(format!("force_metadent_type = ${idx}"));
                info_str_vals.push(v.clone());
                idx += 1;
            }
            None => {}
        }
        if let Some(v) = board.base_thread_creation_span_sec {
            info_sets.push(format!("base_thread_creation_span_sec = ${idx}"));
            info_int_vals.push(v as i32);
            idx += 1;
        }
        if let Some(v) = board.base_response_creation_span_sec {
            info_sets.push(format!("base_response_creation_span_sec = ${idx}"));
            info_int_vals.push(v as i32);
            idx += 1;
        }
        if let Some(v) = board.max_thread_name_byte_length {
            info_sets.push(format!("max_thread_name_byte_length = ${idx}"));
            info_int_vals.push(v as i32);
            idx += 1;
        }
        if let Some(v) = board.max_author_name_byte_length {
            info_sets.push(format!("max_author_name_byte_length = ${idx}"));
            info_int_vals.push(v as i32);
            idx += 1;
        }
        if let Some(v) = board.max_email_byte_length {
            info_sets.push(format!("max_email_byte_length = ${idx}"));
            info_int_vals.push(v as i32);
            idx += 1;
        }
        if let Some(v) = board.max_response_body_byte_length {
            info_sets.push(format!("max_response_body_byte_length = ${idx}"));
            info_int_vals.push(v as i32);
            idx += 1;
        }
        if let Some(v) = board.max_response_body_lines {
            info_sets.push(format!("max_response_body_lines = ${idx}"));
            info_int_vals.push(v as i32);
            idx += 1;
        }
        if let Some(v) = board.threads_archive_trigger_thread_count {
            info_sets.push(format!("threads_archive_trigger_thread_count = ${idx}"));
            info_int_vals.push(v as i32);
            idx += 1;
        }
        if board.read_only.is_some() {
            info_sets.push(format!("read_only = ${idx}"));
            info_bool_val = board.read_only;
            idx += 1;
        }
        // WHERE board_key uses the next param index
        let info_where_idx = idx;

        let mut tx = pool.begin().await?;

        if !info_sets.is_empty() {
            let sql = format!(
                "UPDATE boards_info SET {} WHERE id = (SELECT id FROM boards WHERE board_key = ${})",
                info_sets.join(", "),
                info_where_idx,
            );
            let mut q = sqlx::query(&sql);
            for v in &info_str_vals {
                q = q.bind(v);
            }
            for v in &info_int_vals {
                q = q.bind(v);
            }
            if let Some(v) = info_bool_val {
                q = q.bind(v);
            }
            q = q.bind(board_key);
            q.execute(&mut *tx).await?;
        }

        if !board_sets.is_empty() {
            let where_idx = board_sets.len() + 1;
            let sql = format!(
                "UPDATE boards SET {} WHERE board_key = ${}",
                board_sets.join(", "),
                where_idx,
            );
            let mut q = sqlx::query(&sql);
            for v in &board_str_vals {
                q = q.bind(v);
            }
            q = q.bind(board_key);
            q.execute(&mut *tx).await?;
        }

        tx.commit().await?;

        self.get_boards_by_key(Some(vec![board_key.to_string()]))
            .await?
            .first()
            .cloned()
            .ok_or(anyhow::anyhow!("Failed to edit board"))
    }
}
