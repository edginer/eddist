use std::{
    collections::HashMap,
    sync::{OnceLock, RwLock},
};

use eddist_core::cache_aside::AsCacheRef;
use uuid::Uuid;

use crate::{
    domain::board::{Board, BoardInfo},
    repositories::bbs_repository::BbsRepository,
};

struct BoardInfoLocalCacheItem {
    board_info: BoardInfo,
    expired_at: u64,
}

impl AsCacheRef<BoardInfo> for BoardInfoLocalCacheItem {
    fn expired_at(&self) -> u64 {
        self.expired_at
    }

    fn get(&self) -> &BoardInfo {
        &self.board_info
    }
}

pub struct BoardInfoService<T: BbsRepository>(T);

impl<T: BbsRepository + Clone> BoardInfoService<T> {
    pub fn new(repo: T) -> Self {
        Self(repo)
    }

    pub async fn get_board_info_by_id(&self, board_id: Uuid) -> anyhow::Result<Option<BoardInfo>> {
        static BOARD_INFO_LOCAL_CACHE: OnceLock<RwLock<HashMap<Uuid, BoardInfoLocalCacheItem>>> =
            OnceLock::new();
        let cache = BOARD_INFO_LOCAL_CACHE.get_or_init(|| RwLock::new(HashMap::new()));

        match cache.read() {
            Ok(cache) => {
                if let Some(cache) = cache.get(&board_id) {
                    if cache.expired_at() > chrono::Utc::now().timestamp() as u64 {
                        return Ok(Some(cache.get().clone()));
                    }
                }
            }
            Err(e) => return Err(anyhow::anyhow!("failed to read cache: {e:?}")),
        }

        let board_info = self
            .0
            .get_board_info(board_id)
            .await
            .map_err(|_| anyhow::anyhow!("failed to find board info"))?;
        if board_info.is_none() {
            return Ok(None);
        }

        let mut write_lock = cache
            .write()
            .map_err(|e| anyhow::anyhow!("failed to write cache: {e:?}"))?;

        write_lock.insert(
            board_id,
            BoardInfoLocalCacheItem {
                board_info: board_info.clone().unwrap(),
                expired_at: chrono::Utc::now().timestamp() as u64 + 60,
            },
        );

        Ok(board_info)
    }

    pub async fn get_board_info_by_key(
        &self,
        board_key: &str,
    ) -> anyhow::Result<Option<(Board, BoardInfo)>> {
        let board = self
            .0
            .get_board(board_key)
            .await
            .map_err(|_| anyhow::anyhow!("failed to find board info"))?
            .ok_or_else(|| anyhow::anyhow!("failed to find board info"))?;

        let board_info = self
            .get_board_info_by_id(board.id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("failed to find board info"))?;

        Ok(Some((board, board_info)))
    }
}
