use crate::{
    domain::thread_list::ThreadList,
    repositories::bbs_repository::{BbsRepository, ThreadStatus},
};

use super::AppService;

#[derive(Debug, Clone)]
pub struct ThreadListService<T: BbsRepository>(T);

impl<T: BbsRepository> ThreadListService<T> {
    pub fn new(repo: T) -> Self {
        Self(repo)
    }
}

#[async_trait::async_trait]
impl<T: BbsRepository> AppService<BoardKey, ThreadList> for ThreadListService<T> {
    async fn execute(&self, input: BoardKey) -> anyhow::Result<ThreadList> {
        let board = self
            .0
            .get_board_info(&input.0)
            .await?
            .ok_or_else(|| anyhow::anyhow!("failed to find board info"))?;

        let threads = self
            .0
            .get_threads(board.id, ThreadStatus::Unarchived)
            .await?;

        Ok(ThreadList {
            board,
            thread_list: threads,
        })
    }
}

#[derive(Debug, Clone)]
pub struct BoardKey(pub String);
