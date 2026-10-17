use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use eddist_core::{
    cache_aside::AsCacheRef,
    domain::board::{Board, BoardInfo},
};
use uuid::Uuid;

use crate::repositories::bbs_repository::BbsRepository;

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

#[derive(Clone, Default)]
pub struct BoardInfoCache(Arc<RwLock<HashMap<Uuid, BoardInfoLocalCacheItem>>>);

pub struct BoardInfoService<T: BbsRepository>(T, BoardInfoCache);

impl<T: BbsRepository + Clone> BoardInfoService<T> {
    pub fn new(repo: T, cache: BoardInfoCache) -> Self {
        Self(repo, cache)
    }

    pub async fn get_board_info_by_id(&self, board_id: Uuid) -> anyhow::Result<Option<BoardInfo>> {
        let cache = &self.1.0;

        match cache.read() {
            Ok(cache) => {
                if let Some(cache) = cache.get(&board_id)
                    && cache.expired_at() > chrono::Utc::now().timestamp() as u64
                {
                    return Ok(Some(cache.get().clone()));
                }
            }
            Err(e) => return Err(anyhow::anyhow!("failed to read cache: {e:?}")),
        }

        let board_info = self.0.get_board_info(board_id).await?;
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
        let Some(board) = self.0.get_board(board_key).await? else {
            return Ok(None);
        };
        let board_info = self.get_board_info_by_id(board.id).await?;

        Ok(board_info.map(|board_info| (board, board_info)))
    }
}
