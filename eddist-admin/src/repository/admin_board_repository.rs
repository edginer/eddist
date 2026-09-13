use crate::entity::{board, board_info, thread};
use crate::models::{Board, BoardInfo, CreateBoardInput, EditBoardInput};
use crate::repository::support::empty_to_none;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    FromQueryResult, IntoActiveValue, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
    TransactionTrait,
};
use std::collections::HashMap;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait AdminBoardRepository: Send + Sync {
    async fn get_boards_by_key(&self, keys: Option<Vec<String>>) -> anyhow::Result<Vec<Board>>;
    async fn get_board_info(&self, id: Uuid) -> anyhow::Result<BoardInfo>;
    async fn create_board(&self, board: CreateBoardInput) -> anyhow::Result<Board>;
    async fn edit_board(&self, board_key: &str, board: EditBoardInput) -> anyhow::Result<Board>;
}

#[derive(Clone)]
pub struct AdminBoardRepositoryImpl(DatabaseConnection);

impl AdminBoardRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self(db)
    }
}

#[derive(Debug, FromQueryResult)]
struct ThreadCountByBoard {
    board_id: Uuid,
    thread_count: i64,
}

fn into_board(model: board::Model, thread_count: i64) -> Board {
    Board {
        id: model.id,
        name: model.name,
        board_key: model.board_key,
        default_name: model.default_name,
        thread_count,
    }
}

fn into_board_info(model: board_info::Model) -> BoardInfo {
    BoardInfo {
        local_rules: model.local_rules,
        base_thread_creation_span_sec: model.base_thread_creation_span_sec as usize,
        base_response_creation_span_sec: model.base_response_creation_span_sec as usize,
        max_thread_name_byte_length: model.max_thread_name_byte_length as usize,
        max_author_name_byte_length: model.max_author_name_byte_length as usize,
        max_email_byte_length: model.max_email_byte_length as usize,
        max_response_body_byte_length: model.max_response_body_byte_length as usize,
        max_response_body_lines: model.max_response_body_lines as usize,
        threads_archive_cron: model.threads_archive_cron,
        threads_archive_trigger_thread_count: model
            .threads_archive_trigger_thread_count
            .map(|value| value as usize),
        read_only: model.read_only,
        force_metadent_type: model.force_metadent_type,
        enable_1001_message: model.enable_1001_message,
        custom_1001_message: model.custom_1001_message,
    }
}

impl AdminBoardRepositoryImpl {
    async fn thread_count(&self, board_id: Uuid) -> anyhow::Result<i64> {
        Ok(thread::Entity::find()
            .filter(thread::Column::BoardId.eq(board_id))
            .count(&self.0)
            .await? as i64)
    }
}

#[async_trait::async_trait]
impl AdminBoardRepository for AdminBoardRepositoryImpl {
    async fn get_boards_by_key(&self, keys: Option<Vec<String>>) -> anyhow::Result<Vec<Board>> {
        let mut query = board::Entity::find();
        if let Some(keys) = keys {
            query = query.filter(board::Column::BoardKey.is_in(keys));
        }
        let models = query
            .order_by_asc(board::Column::BoardKey)
            .all(&self.0)
            .await?;

        let board_ids = models.iter().map(|model| model.id).collect::<Vec<_>>();
        let thread_counts = thread::Entity::find()
            .select_only()
            .column(thread::Column::BoardId)
            .column_as(thread::Column::Id.count(), "thread_count")
            .filter(thread::Column::BoardId.is_in(board_ids))
            .group_by(thread::Column::BoardId)
            .into_model::<ThreadCountByBoard>()
            .all(&self.0)
            .await?
            .into_iter()
            .map(|row| (row.board_id, row.thread_count))
            .collect::<HashMap<_, _>>();

        Ok(models
            .into_iter()
            .map(|model| {
                let thread_count = thread_counts.get(&model.id).copied().unwrap_or_default();
                into_board(model, thread_count)
            })
            .collect())
    }

    async fn get_board_info(&self, id: Uuid) -> anyhow::Result<BoardInfo> {
        board_info::Entity::find_by_id(id)
            .one(&self.0)
            .await?
            .map(into_board_info)
            .ok_or_else(|| anyhow::anyhow!("Board info not found: {id}"))
    }

    async fn create_board(&self, board: CreateBoardInput) -> anyhow::Result<Board> {
        let board_id = Uuid::now_v7();
        let now = crate::db_time::now();
        let tx = self.0.begin().await?;

        let created = board::ActiveModel {
            id: Set(board_id),
            name: Set(board.name),
            board_key: Set(board.board_key),
            default_name: Set(board.default_name),
        }
        .insert(&tx)
        .await?;

        board_info::ActiveModel {
            id: Set(board_id),
            local_rules: Set(board.local_rule),
            created_at: Set(now),
            updated_at: Set(now),
            base_thread_creation_span_sec: board
                .base_thread_creation_span_sec
                .map(|value| value as i32)
                .into_active_value(),
            base_response_creation_span_sec: board
                .base_response_creation_span_sec
                .map(|value| value as i32)
                .into_active_value(),
            max_thread_name_byte_length: board
                .max_thread_name_byte_length
                .map(|value| value as i32)
                .into_active_value(),
            max_author_name_byte_length: board
                .max_author_name_byte_length
                .map(|value| value as i32)
                .into_active_value(),
            max_email_byte_length: board
                .max_email_byte_length
                .map(|value| value as i32)
                .into_active_value(),
            max_response_body_byte_length: board
                .max_response_body_byte_length
                .map(|value| value as i32)
                .into_active_value(),
            max_response_body_lines: board
                .max_response_body_lines
                .map(|value| value as i32)
                .into_active_value(),
            threads_archive_cron: board.threads_archive_cron.map(Some).into_active_value(),
            threads_archive_trigger_thread_count: board
                .threads_archive_trigger_thread_count
                .map(|value| Some(value as i32))
                .into_active_value(),
            force_metadent_type: board.force_metadent_type.map(Some).into_active_value(),
            ..Default::default()
        }
        .insert(&tx)
        .await?;

        tx.commit().await?;

        Ok(into_board(created, 0))
    }

    async fn edit_board(&self, board_key: &str, input: EditBoardInput) -> anyhow::Result<Board> {
        let board_model = board::Entity::find()
            .filter(board::Column::BoardKey.eq(board_key))
            .one(&self.0)
            .await?
            .ok_or_else(|| {
                crate::error::ServiceError::NotFound(format!("Board not found: {board_key}"))
            })?;

        let tx = self.0.begin().await?;

        board_info::ActiveModel {
            id: Set(board_model.id),
            local_rules: input.local_rule.into_active_value(),
            threads_archive_cron: input
                .threads_archive_cron
                .map(empty_to_none)
                .into_active_value(),
            force_metadent_type: input
                .force_metadent_type
                .map(empty_to_none)
                .into_active_value(),
            custom_1001_message: input
                .custom_1001_message
                .map(empty_to_none)
                .into_active_value(),
            base_thread_creation_span_sec: input
                .base_thread_creation_span_sec
                .map(|value| value as i32)
                .into_active_value(),
            base_response_creation_span_sec: input
                .base_response_creation_span_sec
                .map(|value| value as i32)
                .into_active_value(),
            max_thread_name_byte_length: input
                .max_thread_name_byte_length
                .map(|value| value as i32)
                .into_active_value(),
            max_author_name_byte_length: input
                .max_author_name_byte_length
                .map(|value| value as i32)
                .into_active_value(),
            max_email_byte_length: input
                .max_email_byte_length
                .map(|value| value as i32)
                .into_active_value(),
            max_response_body_byte_length: input
                .max_response_body_byte_length
                .map(|value| value as i32)
                .into_active_value(),
            max_response_body_lines: input
                .max_response_body_lines
                .map(|value| value as i32)
                .into_active_value(),
            threads_archive_trigger_thread_count: input
                .threads_archive_trigger_thread_count
                .map(|value| Some(value as i32))
                .into_active_value(),
            read_only: input.read_only.into_active_value(),
            enable_1001_message: input.enable_1001_message.into_active_value(),
            ..Default::default()
        }
        .update(&tx)
        .await?;

        let updated = board::ActiveModel {
            id: Set(board_model.id),
            name: input.name.into_active_value(),
            default_name: input.default_name.into_active_value(),
            ..Default::default()
        }
        .update(&tx)
        .await?;

        tx.commit().await?;

        let thread_count = self.thread_count(updated.id).await?;
        Ok(into_board(updated, thread_count))
    }
}
