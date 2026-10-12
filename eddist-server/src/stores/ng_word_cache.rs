use eddist_core::cache_aside::{AsCache, ToCache, cache_aside};
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};

use crate::{domain::ng_word::NgWord, repositories::bbs_repository::BbsRepository};

const NG_WORD_CACHE_TTL_SECS: u64 = 120;

#[async_trait::async_trait]
pub trait NgWordCache: Send + Sync {
    async fn get_ng_words(&self, board_key: &str) -> anyhow::Result<Vec<NgWord>>;
}

#[derive(Clone)]
pub struct RedisNgWordCache<T: BbsRepository>(T, ConnectionManager);

impl<T: BbsRepository> RedisNgWordCache<T> {
    pub fn new(repo: T, redis_conn: ConnectionManager) -> Self {
        Self(repo, redis_conn)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NgWordCacheEntry {
    ng_words: Vec<NgWord>,
    expired_at: u64,
}

impl AsCache<Vec<NgWord>> for NgWordCacheEntry {
    fn expired_at(&self) -> u64 {
        self.expired_at
    }

    fn get(self) -> Vec<NgWord> {
        self.ng_words
    }
}

impl ToCache<Vec<NgWord>, NgWordCacheEntry> for Vec<NgWord> {
    fn into_cache(self, expired_at: u64) -> NgWordCacheEntry {
        NgWordCacheEntry {
            ng_words: self,
            expired_at,
        }
    }
}

#[async_trait::async_trait]
impl<T: BbsRepository + Clone + 'static> NgWordCache for RedisNgWordCache<T> {
    async fn get_ng_words(&self, board_key: &str) -> anyhow::Result<Vec<NgWord>> {
        let board_key = board_key.to_string();
        let repo = self.0.clone();
        let expired_at = chrono::Utc::now().timestamp() as u64 + NG_WORD_CACHE_TTL_SECS;
        cache_aside::<NgWordCacheEntry, _, _, _>(
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
    }
}
