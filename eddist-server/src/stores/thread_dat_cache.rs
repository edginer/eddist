use anyhow::anyhow;
use eddist_core::redis_keys::thread_cache_key;
use redis::{AsyncCommands, Cmd, Value, aio::ConnectionManager};

const CREATED_THREAD_TTL_SECS: i64 = 60 * 60 * 24 * 7;

#[async_trait::async_trait]
pub trait ThreadDatCache: Send + Sync {
    /// Returns an empty list when the thread is not cached.
    async fn read(&self, board_key: &str, thread_number: u64) -> anyhow::Result<Vec<Vec<u8>>>;

    /// Returns the new line count, or `None` without writing when the thread is not cached.
    async fn append_if_cached(
        &self,
        board_key: &str,
        thread_number: u64,
        line: Vec<u8>,
    ) -> anyhow::Result<Option<u64>>;

    async fn create(
        &self,
        board_key: &str,
        thread_number: u64,
        first_line: Vec<u8>,
    ) -> anyhow::Result<()>;
}

#[derive(Clone)]
pub struct RedisThreadDatCache(ConnectionManager);

impl RedisThreadDatCache {
    pub fn new(redis_conn: ConnectionManager) -> Self {
        Self(redis_conn)
    }
}

#[async_trait::async_trait]
impl ThreadDatCache for RedisThreadDatCache {
    async fn read(&self, board_key: &str, thread_number: u64) -> anyhow::Result<Vec<Vec<u8>>> {
        let mut conn = self.0.clone();
        Ok(conn
            .lrange(thread_cache_key(board_key, thread_number), 0, -1)
            .await?)
    }

    async fn append_if_cached(
        &self,
        board_key: &str,
        thread_number: u64,
        line: Vec<u8>,
    ) -> anyhow::Result<Option<u64>> {
        let mut conn = self.0.clone();
        // RPUSHX checks existence and appends in one command, so an expiring key cannot be
        // recreated without its TTL.
        let Value::Int(len) = conn
            .send_packed_command(&Cmd::rpush_exists(
                thread_cache_key(board_key, thread_number),
                line,
            ))
            .await?
        else {
            return Err(anyhow!("failed to parse redis response"));
        };
        Ok((len > 0).then_some(len as u64))
    }

    async fn create(
        &self,
        board_key: &str,
        thread_number: u64,
        first_line: Vec<u8>,
    ) -> anyhow::Result<()> {
        let mut conn = self.0.clone();
        let key = thread_cache_key(board_key, thread_number);
        conn.send_packed_command(&Cmd::rpush(&key, first_line))
            .await?;
        conn.send_packed_command(&Cmd::expire(&key, CREATED_THREAD_TTL_SECS))
            .await?;
        Ok(())
    }
}
