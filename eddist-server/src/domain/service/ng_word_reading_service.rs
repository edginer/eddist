use eddist_core::cache_aside::{cache_aside, AsCache};
use redis::aio::MultiplexedConnection;
use serde::{Deserialize, Serialize};

use crate::{
    domain::ng_word::NgWord, error::BbsCgiError, repositories::bbs_repository::BbsRepository,
};

#[derive(Clone)]
pub struct NgWordReadingService<T: BbsRepository>(T, MultiplexedConnection);

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

impl<T: BbsRepository + Clone> NgWordReadingService<T> {
    pub fn new(repo: T, redis_conn: MultiplexedConnection) -> Self {
        Self(repo, redis_conn)
    }

    pub async fn get_ng_words(&self, board_key: &str) -> Result<Vec<NgWord>, BbsCgiError> {
        let board_key = board_key.to_string();
        let repo = self.0.clone();
        cache_aside::<NgWordCache, _, _>(&board_key, "ng_words", &mut self.1.clone(), || {
            let board_key = board_key.clone();
            Box::pin(async move { repo.get_ng_words_by_board_key(&board_key).await })
        })
        .await
        .map_err(BbsCgiError::Other)
    }
}
