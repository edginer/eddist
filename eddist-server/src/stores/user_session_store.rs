use eddist_core::{redis_keys::user_session_key, utils::to_hex};
use redis::{AsyncCommands, aio::ConnectionManager};
use sha3::Digest;
use uuid::Uuid;

const USER_SESSION_TTL_SECS: u64 = 60 * 60 * 24 * 365;

#[async_trait::async_trait]
pub trait UserSessionStore: Send + Sync {
    /// Returns the stored user ID unparsed; callers decide how to treat a malformed value.
    async fn get_user_id(&self, user_sid: &str) -> anyhow::Result<Option<String>>;

    async fn exists(&self, user_sid: &str) -> anyhow::Result<bool>;

    /// Returns the new session ID.
    async fn create(&self, user_id: Uuid) -> anyhow::Result<String>;

    async fn delete(&self, user_sid: &str) -> anyhow::Result<()>;
}

#[derive(Clone)]
pub struct RedisUserSessionStore(ConnectionManager);

impl RedisUserSessionStore {
    pub fn new(redis_conn: ConnectionManager) -> Self {
        Self(redis_conn)
    }
}

#[async_trait::async_trait]
impl UserSessionStore for RedisUserSessionStore {
    async fn get_user_id(&self, user_sid: &str) -> anyhow::Result<Option<String>> {
        let mut conn = self.0.clone();
        Ok(conn.get(user_session_key(user_sid)).await?)
    }

    async fn exists(&self, user_sid: &str) -> anyhow::Result<bool> {
        let mut conn = self.0.clone();
        Ok(conn.exists(user_session_key(user_sid)).await?)
    }

    async fn create(&self, user_id: Uuid) -> anyhow::Result<String> {
        let mut hasher = sha3::Sha3_512::new();
        hasher.update(Uuid::now_v7().to_string());
        let user_sid = to_hex(hasher.finalize());

        let mut conn = self.0.clone();
        conn.set_ex::<_, _, ()>(
            user_session_key(&user_sid),
            user_id.to_string(),
            USER_SESSION_TTL_SECS,
        )
        .await?;
        Ok(user_sid)
    }

    async fn delete(&self, user_sid: &str) -> anyhow::Result<()> {
        let mut conn = self.0.clone();
        conn.del::<_, bool>(user_session_key(user_sid)).await?;
        Ok(())
    }
}

#[cfg(test)]
mod probe_tests {
    use super::*;

    /// user_sid is a fresh Redis session key, not a persisted/compared value across
    /// restarts, so only the hex shape needs to stay stable, not the exact digits.
    #[test]
    fn hex_output_shape_stable_across_digest_crate_bump() {
        let mut hasher = sha3::Sha3_512::new();
        hasher.update("fixed-uuid-for-probe");
        let user_sid = to_hex(hasher.finalize());
        assert_eq!(user_sid.len(), 128);
        assert!(user_sid.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
