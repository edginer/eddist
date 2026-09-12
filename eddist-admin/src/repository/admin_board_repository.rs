use crate::entity::{board, board_info, thread};
use crate::models::{Board, BoardInfo, CreateBoardInput, EditBoardInput};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    FromQueryResult, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, TransactionTrait,
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
struct ThreadBoardId {
    board_id: Uuid,
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
    async fn get_board_by_key(&self, board_key: &str) -> anyhow::Result<Option<Board>> {
        let Some(model) = board::Entity::find()
            .filter(board::Column::BoardKey.eq(board_key))
            .one(&self.0)
            .await?
        else {
            return Ok(None);
        };

        let thread_count = thread::Entity::find()
            .filter(thread::Column::BoardId.eq(model.id))
            .count(&self.0)
            .await? as i64;
        Ok(Some(into_board(model, thread_count)))
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

        let thread_board_ids = thread::Entity::find()
            .select_only()
            .column(thread::Column::BoardId)
            .into_model::<ThreadBoardId>()
            .all(&self.0)
            .await?;
        let mut thread_counts = HashMap::<Uuid, i64>::new();
        for row in thread_board_ids {
            *thread_counts.entry(row.board_id).or_default() += 1;
        }

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
        let board_key = board.board_key.clone();
        let now = chrono::Utc::now().naive_utc();
        let tx = self.0.begin().await?;

        board::ActiveModel {
            id: Set(board_id),
            name: Set(board.name),
            board_key: Set(board.board_key),
            default_name: Set(board.default_name),
        }
        .insert(&tx)
        .await?;

        let mut board_info_model = board_info::ActiveModel {
            id: Set(board_id),
            local_rules: Set(board.local_rule),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        if let Some(value) = board.base_thread_creation_span_sec {
            board_info_model.base_thread_creation_span_sec = Set(value as i32);
        }
        if let Some(value) = board.base_response_creation_span_sec {
            board_info_model.base_response_creation_span_sec = Set(value as i32);
        }
        if let Some(value) = board.max_thread_name_byte_length {
            board_info_model.max_thread_name_byte_length = Set(value as i32);
        }
        if let Some(value) = board.max_author_name_byte_length {
            board_info_model.max_author_name_byte_length = Set(value as i32);
        }
        if let Some(value) = board.max_email_byte_length {
            board_info_model.max_email_byte_length = Set(value as i32);
        }
        if let Some(value) = board.max_response_body_byte_length {
            board_info_model.max_response_body_byte_length = Set(value as i32);
        }
        if let Some(value) = board.max_response_body_lines {
            board_info_model.max_response_body_lines = Set(value as i32);
        }
        if let Some(value) = board.threads_archive_cron {
            board_info_model.threads_archive_cron = Set(Some(value));
        }
        if let Some(value) = board.threads_archive_trigger_thread_count {
            board_info_model.threads_archive_trigger_thread_count = Set(Some(value as i32));
        }
        if let Some(value) = board.force_metadent_type {
            board_info_model.force_metadent_type = Set(Some(value));
        }
        board_info_model.insert(&tx).await?;

        tx.commit().await?;

        self.get_board_by_key(&board_key)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to create board"))
    }

    async fn edit_board(&self, board_key: &str, input: EditBoardInput) -> anyhow::Result<Board> {
        let board_model = board::Entity::find()
            .filter(board::Column::BoardKey.eq(board_key))
            .one(&self.0)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Board not found: {board_key}"))?;

        let tx = self.0.begin().await?;
        let mut info_model = board_info::ActiveModel {
            id: Set(board_model.id),
            ..Default::default()
        };
        let mut info_changed = false;

        if let Some(value) = input.local_rule {
            info_model.local_rules = Set(value);
            info_changed = true;
        }
        if let Some(value) = input.threads_archive_cron {
            info_model.threads_archive_cron =
                Set(if value.is_empty() { None } else { Some(value) });
            info_changed = true;
        }
        if let Some(value) = input.force_metadent_type {
            info_model.force_metadent_type = Set(if value.is_empty() { None } else { Some(value) });
            info_changed = true;
        }
        if let Some(value) = input.custom_1001_message {
            info_model.custom_1001_message = Set(if value.is_empty() { None } else { Some(value) });
            info_changed = true;
        }
        if let Some(value) = input.base_thread_creation_span_sec {
            info_model.base_thread_creation_span_sec = Set(value as i32);
            info_changed = true;
        }
        if let Some(value) = input.base_response_creation_span_sec {
            info_model.base_response_creation_span_sec = Set(value as i32);
            info_changed = true;
        }
        if let Some(value) = input.max_thread_name_byte_length {
            info_model.max_thread_name_byte_length = Set(value as i32);
            info_changed = true;
        }
        if let Some(value) = input.max_author_name_byte_length {
            info_model.max_author_name_byte_length = Set(value as i32);
            info_changed = true;
        }
        if let Some(value) = input.max_email_byte_length {
            info_model.max_email_byte_length = Set(value as i32);
            info_changed = true;
        }
        if let Some(value) = input.max_response_body_byte_length {
            info_model.max_response_body_byte_length = Set(value as i32);
            info_changed = true;
        }
        if let Some(value) = input.max_response_body_lines {
            info_model.max_response_body_lines = Set(value as i32);
            info_changed = true;
        }
        if let Some(value) = input.threads_archive_trigger_thread_count {
            info_model.threads_archive_trigger_thread_count = Set(Some(value as i32));
            info_changed = true;
        }
        if let Some(value) = input.read_only {
            info_model.read_only = Set(value);
            info_changed = true;
        }
        if let Some(value) = input.enable_1001_message {
            info_model.enable_1001_message = Set(value);
            info_changed = true;
        }
        if info_changed {
            info_model.update(&tx).await?;
        }

        let mut board_update = board::ActiveModel {
            id: Set(board_model.id),
            ..Default::default()
        };
        let mut board_changed = false;
        if let Some(value) = input.name {
            board_update.name = Set(value);
            board_changed = true;
        }
        if let Some(value) = input.default_name {
            board_update.default_name = Set(value);
            board_changed = true;
        }
        if board_changed {
            board_update.update(&tx).await?;
        }

        tx.commit().await?;

        self.get_board_by_key(board_key)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to edit board"))
    }
}
