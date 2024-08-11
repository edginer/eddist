use crate::{domain::board::BoardInfo, repositories::bbs_repository::BbsRepository};

use super::AppService;

#[derive(Debug, Clone)]
pub struct BoardInfoService<T: BbsRepository>(T);

impl<T: BbsRepository> BoardInfoService<T> {
    pub fn new(repo: T) -> Self {
        Self(repo)
    }
}

#[async_trait::async_trait]
impl<T: BbsRepository> AppService<BoardInfoServiceInput, BoardInfoServiceOutput>
    for BoardInfoService<T>
{
    async fn execute(
        &self,
        input: BoardInfoServiceInput,
    ) -> anyhow::Result<BoardInfoServiceOutput> {
        let board = self
            .0
            .get_board(&input.board_key)
            .await?
            .ok_or_else(|| anyhow::anyhow!("failed to find board info"))?;

        let board_info = self
            .0
            .get_board_info(board.id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("failed to find board info"))?;

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
