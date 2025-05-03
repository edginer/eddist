use eddist_core::cache_aside::{AsCache, ToCache, cache_aside};
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};

use crate::{
    domain::ng_word::NgWord, error::BbsCgiError, repositories::bbs_repository::BbsRepository,
};

#[derive(Clone)]
pub struct NgWordReadingService<T: BbsRepository>(T, ConnectionManager);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NgWordCache {
    ng_words: Vec<NgWord>,
    expired_at: u64,
}

impl AsCache<Vec<NgWord>> for NgWordCache {
    fn expired_at(&self) -> u64 {
        self.expired_at
    }

    fn get(self) -> Vec<NgWord> {
        self.ng_words
    }
}

impl ToCache<Vec<NgWord>, NgWordCache> for Vec<NgWord> {
    fn into_cache(self, expired_at: u64) -> NgWordCache {
        NgWordCache {
            ng_words: self,
            expired_at,
        }
    }
}

impl<T: BbsRepository + Clone> NgWordReadingService<T> {
    pub fn new(repo: T, redis_conn: ConnectionManager) -> Self {
        Self(repo, redis_conn)
    }

    pub async fn get_ng_words(&self, board_key: &str) -> Result<Vec<NgWord>, BbsCgiError> {
        let board_key = board_key.to_string();
        let repo = self.0.clone();
        let expired_at = chrono::Utc::now().timestamp() as u64 + 120;
        cache_aside::<NgWordCache, _, _, _>(
            &board_key,
            "ng_words",
            Box::new(self.1.clone()),
            expired_at,
            || {
                let board_key = board_key.clone();
                Box::pin(async move { repo.get_ng_words_by_board_key(&board_key).await })
            },
        )
        .await
        .map_err(BbsCgiError::Other)
    }
}
