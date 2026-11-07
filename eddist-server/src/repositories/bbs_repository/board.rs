use eddist_core::domain::{
    board::{Board, BoardInfo},
    cap::Cap,
};
use eddist_entity::{board, board_cap, board_info, board_ng_word, cap, ng_word};
use sea_orm::{ColumnTrait, EntityTrait, JoinType, QueryFilter, QuerySelect, RelationTrait};
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

fn into_board(row: board::Model) -> Board {
    Board {
        id: row.id,
        name: row.name,
        board_key: row.board_key,
        default_name: row.default_name,
    }
}

fn into_board_info(row: board_info::Model) -> BoardInfo {
    BoardInfo {
        id: row.id,
        local_rules: row.local_rules,
        base_thread_creation_span_sec: row.base_thread_creation_span_sec,
        base_response_creation_span_sec: row.base_response_creation_span_sec,
        max_thread_name_byte_length: row.max_thread_name_byte_length,
        max_author_name_byte_length: row.max_author_name_byte_length,
        max_email_byte_length: row.max_email_byte_length,
        max_response_body_byte_length: row.max_response_body_byte_length,
        max_response_body_lines: row.max_response_body_lines,
        threads_archive_cron: row.threads_archive_cron,
        threads_archive_trigger_thread_count: row.threads_archive_trigger_thread_count,
        created_at: row.created_at,
        updated_at: row.updated_at,
        read_only: row.read_only,
        force_metadent_type: row.force_metadent_type,
        enable_1001_message: row.enable_1001_message,
        custom_1001_message: row.custom_1001_message,
    }
}

#[async_trait::async_trait]
impl BoardRepository for BbsRepositoryImpl {
    async fn get_boards(&self) -> anyhow::Result<Vec<Board>> {
        Ok(board::Entity::find()
            .all(&self.db)
            .await?
            .into_iter()
            .map(into_board)
            .collect())
    }

    async fn get_board(&self, board_key: &str) -> anyhow::Result<Option<Board>> {
        Ok(board::Entity::find()
            .filter(board::Column::BoardKey.eq(board_key))
            .one(&self.db)
            .await?
            .map(into_board))
    }

    async fn get_board_info(&self, board_id: Uuid) -> anyhow::Result<Option<BoardInfo>> {
        Ok(board_info::Entity::find_by_id(board_id)
            .one(&self.db)
            .await?
            .map(into_board_info))
    }

    async fn get_ng_words_by_board_key(&self, board_key: &str) -> anyhow::Result<Vec<NgWord>> {
        Ok(ng_word::Entity::find()
            .join(JoinType::InnerJoin, ng_word::Relation::BoardNgWord.def())
            .join(JoinType::InnerJoin, board_ng_word::Relation::Board.def())
            .filter(board::Column::BoardKey.eq(board_key))
            .all(&self.db)
            .await?
            .into_iter()
            .map(|row| NgWord {
                id: row.id,
                name: row.name,
                word: row.word,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect())
    }

    async fn get_cap_by_board_key(
        &self,
        cap_hash: &str,
        board_key: &str,
    ) -> anyhow::Result<Option<Cap>> {
        Ok(cap::Entity::find()
            .join(JoinType::InnerJoin, cap::Relation::BoardCap.def())
            .join(JoinType::InnerJoin, board_cap::Relation::Board.def())
            .filter(cap::Column::PasswordHash.eq(cap_hash))
            .filter(board::Column::BoardKey.eq(board_key))
            .one(&self.db)
            .await?
            .map(|row| Cap {
                id: row.id,
                name: row.name,
                password_hash: row.password_hash,
                description: row.description,
                created_at: row.created_at,
                updated_at: row.updated_at,
            }))
    }
}
