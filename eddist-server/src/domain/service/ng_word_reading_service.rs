use redis::{aio::MultiplexedConnection, AsyncCommands};
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

impl<T: BbsRepository> NgWordReadingService<T> {
    pub fn new(repo: T, redis_conn: MultiplexedConnection) -> Self {
        Self(repo, redis_conn)
    }

    pub async fn get_ng_words(&self, board_key: &str) -> Result<Vec<NgWord>, BbsCgiError> {
        // Cache-aside
        let mut redis_conn = self.1.clone();
        let cache_key = format!("ng_words:{board_key}");
        let cache_ng_words = redis_conn
            .get::<_, Option<String>>(&cache_key)
            .await
            .map_err(|e| BbsCgiError::Other(e.into()))?;

        if let Some(cache_ng_words) = cache_ng_words {
            let cache_ng_words = serde_json::from_str::<NgWordCache>(&cache_ng_words);
            match cache_ng_words {
                Ok(c) if c.expired_at > chrono::Utc::now().timestamp() as u64 => {
                    return Ok(c.ng_words)
                }
                Err(_) => {
                    redis_conn
                        .del::<_, u32>(&cache_key)
                        .await
                        .map_err(|ex| BbsCgiError::Other(ex.into()))?;
                }
                _ => {}
            }
        }

        let ng_words = self
            .0
            .get_ng_words_by_board_key(board_key)
            .await
            .map_err(BbsCgiError::Other)?;

        let cache_ng_words = NgWordCache {
            ng_words: ng_words.clone(),
            expired_at: chrono::Utc::now().timestamp() as u64 + 60,
        };

        redis_conn
            .set::<_, _, _>(&cache_key, serde_json::to_string(&cache_ng_words).unwrap())
            .await
            .map_err(|e| BbsCgiError::Other(e.into()))?;

        Ok(ng_words)
    }
}
