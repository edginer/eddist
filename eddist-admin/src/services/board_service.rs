use std::sync::Arc;

use crate::{
    auth::AdminIdentity,
    models::{Board, BoardInfo, CreateBoardInput, EditBoardInput},
    repository::admin_board_repository::AdminBoardRepository,
};

#[async_trait::async_trait]
pub trait BoardService: Send + Sync {
    async fn get_boards(&self, keys: Option<Vec<String>>) -> anyhow::Result<Vec<Board>>;
    async fn get_board_info(&self, board_id: uuid::Uuid) -> anyhow::Result<BoardInfo>;
    async fn create_board(
        &self,
        actor: &AdminIdentity,
        input: CreateBoardInput,
    ) -> anyhow::Result<Board>;
    async fn edit_board(
        &self,
        actor: &AdminIdentity,
        board_key: &str,
        input: EditBoardInput,
    ) -> anyhow::Result<Board>;
}

pub struct BoardServiceImpl {
    repo: Arc<dyn AdminBoardRepository>,
}

impl BoardServiceImpl {
    pub fn new(repo: Arc<dyn AdminBoardRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait::async_trait]
impl BoardService for BoardServiceImpl {
    async fn get_boards(&self, keys: Option<Vec<String>>) -> anyhow::Result<Vec<Board>> {
        self.repo.get_boards_by_key(keys).await
    }

    async fn get_board_info(&self, board_id: uuid::Uuid) -> anyhow::Result<BoardInfo> {
        self.repo.get_board_info(board_id).await
    }

    async fn create_board(
        &self,
        _actor: &AdminIdentity,
        input: CreateBoardInput,
    ) -> anyhow::Result<Board> {
        self.repo.create_board(input).await
    }

    async fn edit_board(
        &self,
        _actor: &AdminIdentity,
        board_key: &str,
        input: EditBoardInput,
    ) -> anyhow::Result<Board> {
        self.repo.edit_board(board_key, input).await
    }
}
