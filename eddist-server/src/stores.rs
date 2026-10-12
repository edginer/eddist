use std::sync::Arc;

use redis::aio::ConnectionManager;

use crate::repositories::bbs_repository::BbsRepository;

pub mod auth_state_store;
pub mod ng_word_cache;
pub mod oidc_state_store;
pub mod posting_rate_limit_store;
pub mod shared_ng_store;
pub mod thread_dat_cache;
pub mod user_session_store;

use auth_state_store::{AuthStateStore, RedisAuthStateStore};
use ng_word_cache::{NgWordCache, RedisNgWordCache};
use posting_rate_limit_store::{PostingRateLimitStore, RedisPostingRateLimitStore};
use thread_dat_cache::{RedisThreadDatCache, ThreadDatCache};

#[derive(Clone)]
pub struct PostingStores {
    pub dat_cache: Arc<dyn ThreadDatCache>,
    pub rate_limit: Arc<dyn PostingRateLimitStore>,
    pub auth_state: Arc<dyn AuthStateStore>,
    pub ng_words: Arc<dyn NgWordCache>,
}

impl PostingStores {
    pub fn redis<B: BbsRepository + Clone + 'static>(
        redis_conn: ConnectionManager,
        bbs_repo: B,
    ) -> Self {
        Self {
            dat_cache: Arc::new(RedisThreadDatCache::new(redis_conn.clone())),
            rate_limit: Arc::new(RedisPostingRateLimitStore::new(redis_conn.clone())),
            auth_state: Arc::new(RedisAuthStateStore::new(redis_conn.clone())),
            ng_words: Arc::new(RedisNgWordCache::new(bbs_repo, redis_conn)),
        }
    }
}
