use eddist_core::domain::{
    board::{Board, BoardInfo},
    cap::Cap,
};
use sqlx::query_as;
use uuid::Uuid;

use crate::domain::ng_word::NgWord;

use super::BbsRepositoryImpl;

#[async_trait::async_trait]
pub trait BoardRepository: Send + Sync + 'static {
    async fn get_boards(&self) -> anyhow::Result<Vec<Board>>;
    async fn get_board(&self, board_key: &str) -> anyhow::Result<Option<Board>>;
    async fn get_board_info(&self, board_id: Uuid) -> anyhow::Result<Option<BoardInfo>>;
    async fn get_ng_words_by_board_key(&self, board_key: &str) -> anyhow::Result<Vec<NgWord>>;
    async fn get_cap_by_board_key(
        &self,
        cap_hash: &str,
        board_key: &str,
    ) -> anyhow::Result<Option<Cap>>;
}

#[async_trait::async_trait]
impl BoardRepository for BbsRepositoryImpl {
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
        let board = query_as!(
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
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(board.map(|x| Board {
            id: x.id,
            name: x.name,
            board_key: x.board_key,
            default_name: x.default_name,
        }))
    }

    async fn get_board_info(&self, board_id: Uuid) -> anyhow::Result<Option<BoardInfo>> {
        Ok(query_as!(
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
            force_metadent_type,
            enable_1001_message AS "enable_1001_message: bool",
            custom_1001_message
        FROM boards_info
        WHERE id = ?
        "#,
            board_id
        )
        .fetch_optional(&self.pool)
        .await?)
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
