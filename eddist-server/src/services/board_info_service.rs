use crate::{
    domain::{self, board::BoardInfo},
    repositories::bbs_repository::BbsRepository,
};

use super::AppService;

#[derive(Debug, Clone)]
pub struct BoardInfoService<T: BbsRepository>(T);

impl<T: BbsRepository> BoardInfoService<T> {
    pub fn new(repo: T) -> Self {
        Self(repo)
    }
}

#[async_trait::async_trait]
impl<T: BbsRepository + Clone> AppService<BoardInfoServiceInput, BoardInfoServiceOutput>
    for BoardInfoService<T>
{
    async fn execute(
        &self,
        input: BoardInfoServiceInput,
    ) -> anyhow::Result<BoardInfoServiceOutput> {
        let svc = domain::service::board_info_service::BoardInfoService::new(self.0.clone());

        let (board, board_info) = svc
            .get_board_info_by_key(&input.board_key)
            .await?
            .ok_or_else(|| anyhow::anyhow!("board not found"))?;

        Ok(BoardInfoServiceOutput {
            board_key: board.board_key,
            name: board.name,
            default_name: board.default_name,
            board_info,
        })
    }
}

pub struct BoardInfoServiceInput {
    pub board_key: String,
}

pub struct BoardInfoServiceOutput {
    pub board_key: String,
    pub name: String,
    pub default_name: String,
    pub board_info: BoardInfo,
}
