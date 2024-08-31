use serde::{Deserialize, Serialize};

use crate::repositories::bbs_repository::BbsRepository;

use super::AppService;

#[derive(Debug, Clone)]
pub struct ListBoardsService<T: BbsRepository>(T);

impl<T: BbsRepository> ListBoardsService<T> {
    pub fn new(repo: T) -> Self {
        Self(repo)
    }
}

#[async_trait::async_trait]
impl<T: BbsRepository + Clone> AppService<(), Vec<ListBoardsServiceOutputBoard>>
    for ListBoardsService<T>
{
    async fn execute(&self, _input: ()) -> anyhow::Result<Vec<ListBoardsServiceOutputBoard>> {
        let boards = self.0.get_boards().await?;
        Ok(boards
            .into_iter()
            .map(|board| ListBoardsServiceOutputBoard {
                name: board.name,
                board_key: board.board_key,
                default_name: board.default_name,
            })
            .collect())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListBoardsServiceOutputBoard {
    pub name: String,
    pub board_key: String,
    pub default_name: String,
}
