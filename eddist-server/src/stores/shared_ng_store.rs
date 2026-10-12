use eddist_core::redis_keys::{
    shared_ng_id_active_key, shared_ng_id_key, shared_ng_id_rate_limit_key,
    shared_ng_thread_metadent_active_key, shared_ng_thread_metadent_key,
};
use redis::{aio::ConnectionManager, pipe};

use crate::utils::incr_fixed_window;

// Shared NG marks are intentionally short-lived. They represent recent community
// signals rather than permanent moderation decisions.
const SHARED_NG_TTL_SECS: i64 = 3 * 24 * 60 * 60;

const CONTRIBUTION_QUOTA: i64 = 25;
const CONTRIBUTION_QUOTA_WINDOW_SECS: i64 = 24 * 60 * 60;

#[derive(Debug, Clone, Copy)]
pub enum SharedNgKind {
    AuthorId,
    ThreadMetadent,
}

impl SharedNgKind {
    fn contributors_key(self, board_key: &str, value: &str) -> String {
        match self {
            SharedNgKind::AuthorId => shared_ng_id_key(board_key, value),
            SharedNgKind::ThreadMetadent => shared_ng_thread_metadent_key(board_key, value),
        }
    }

    fn active_key(self, board_key: &str) -> String {
        match self {
            SharedNgKind::AuthorId => shared_ng_id_active_key(board_key),
            SharedNgKind::ThreadMetadent => shared_ng_thread_metadent_active_key(board_key),
        }
    }
}

pub struct ActiveSharedNg {
    pub author_ids: Vec<String>,
    pub thread_metadents: Vec<String>,
}

#[async_trait::async_trait]
pub trait SharedNgStore: Send + Sync {
    /// Counts one mutation against the contributor's daily quota, which both kinds share
    /// so adding the other kind cannot bypass it. Returns whether the mutation is allowed.
    async fn take_contribution_quota(&self, contributor: &str) -> anyhow::Result<bool>;

    /// Returns the number of distinct contributors after adding this one.
    async fn add_contributor(
        &self,
        kind: SharedNgKind,
        board_key: &str,
        value: &str,
        contributor: &str,
    ) -> anyhow::Result<usize>;

    /// Serves `value` board-wide until it goes unrefreshed for the shared NG TTL.
    async fn promote(
        &self,
        kind: SharedNgKind,
        board_key: &str,
        value: &str,
        now: i64,
    ) -> anyhow::Result<()>;

    /// Does not demote values that were already promoted.
    async fn remove_contributor(
        &self,
        kind: SharedNgKind,
        board_key: &str,
        values: &[String],
        contributor: &str,
    ) -> anyhow::Result<()>;

    /// Drops expired promotions and returns up to `limit` of each kind, newest first.
    async fn active(
        &self,
        board_key: &str,
        now: i64,
        limit: isize,
    ) -> anyhow::Result<ActiveSharedNg>;
}

#[derive(Clone)]
pub struct RedisSharedNgStore(ConnectionManager);

impl RedisSharedNgStore {
    pub fn new(redis_conn: ConnectionManager) -> Self {
        Self(redis_conn)
    }
}

#[async_trait::async_trait]
impl SharedNgStore for RedisSharedNgStore {
    async fn take_contribution_quota(&self, contributor: &str) -> anyhow::Result<bool> {
        let mut conn = self.0.clone();
        let count = incr_fixed_window(
            &mut conn,
            &shared_ng_id_rate_limit_key(contributor),
            CONTRIBUTION_QUOTA_WINDOW_SECS,
        )
        .await?;
        Ok(count <= CONTRIBUTION_QUOTA)
    }

    async fn add_contributor(
        &self,
        kind: SharedNgKind,
        board_key: &str,
        value: &str,
        contributor: &str,
    ) -> anyhow::Result<usize> {
        let mut conn = self.0.clone();
        let key = kind.contributors_key(board_key, value);
        let (contributors,) = pipe()
            .sadd(&key, contributor)
            .ignore()
            .expire(&key, SHARED_NG_TTL_SECS)
            .ignore()
            .scard(&key)
            .query_async::<(usize,)>(&mut conn)
            .await?;
        Ok(contributors)
    }

    async fn promote(
        &self,
        kind: SharedNgKind,
        board_key: &str,
        value: &str,
        now: i64,
    ) -> anyhow::Result<()> {
        let mut conn = self.0.clone();
        let active_key = kind.active_key(board_key);
        pipe()
            .zadd(&active_key, value, now)
            .ignore()
            .expire(&active_key, SHARED_NG_TTL_SECS)
            .ignore()
            .query_async::<()>(&mut conn)
            .await?;
        Ok(())
    }

    async fn remove_contributor(
        &self,
        kind: SharedNgKind,
        board_key: &str,
        values: &[String],
        contributor: &str,
    ) -> anyhow::Result<()> {
        let mut conn = self.0.clone();
        let mut pipe = pipe();
        for value in values {
            // SREM is a no-op if the member/key is missing.
            pipe.srem(kind.contributors_key(board_key, value), contributor)
                .ignore();
        }
        pipe.query_async::<()>(&mut conn).await?;
        Ok(())
    }

    async fn active(
        &self,
        board_key: &str,
        now: i64,
        limit: isize,
    ) -> anyhow::Result<ActiveSharedNg> {
        let mut conn = self.0.clone();
        let author_id_key = SharedNgKind::AuthorId.active_key(board_key);
        let metadent_key = SharedNgKind::ThreadMetadent.active_key(board_key);
        // Promotions are only refreshed on contribution, so entries older than the
        // contributor sets' own TTL are dropped here rather than being served forever.
        let cutoff = now - SHARED_NG_TTL_SECS;
        let last = limit - 1;

        let (author_ids, thread_metadents) = pipe()
            .zrembyscore(&author_id_key, "-inf", cutoff)
            .ignore()
            .zrembyscore(&metadent_key, "-inf", cutoff)
            .ignore()
            .zrevrange(&author_id_key, 0, last)
            .zrevrange(&metadent_key, 0, last)
            .query_async::<(Vec<String>, Vec<String>)>(&mut conn)
            .await?;

        Ok(ActiveSharedNg {
            author_ids,
            thread_metadents,
        })
    }
}
