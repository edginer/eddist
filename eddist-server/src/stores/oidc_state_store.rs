use eddist_core::redis_keys::{
    user_login_oauth2_authreq_key, user_reg_oauth2_authreq_key, user_reg_oauth2_state_key,
};
use redis::{AsyncCommands, aio::ConnectionManager};
use serde::de::DeserializeOwned;

use crate::domain::user::{user_login_state::UserLoginState, user_reg_state::UserRegState};

const USER_REG_STATE_TTL_SECS: u64 = 60 * 3;
const AUTHREQ_TTL_SECS: u64 = 60 * 15;

/// Each `take_*` consumes the state, so a state ID can be used at most once.
#[async_trait::async_trait]
pub trait OidcStateStore: Send + Sync {
    /// Registration state before an IdP is chosen.
    async fn save_user_reg_state(&self, state_id: &str, state: &UserRegState)
    -> anyhow::Result<()>;

    async fn take_user_reg_state(&self, state_id: &str) -> anyhow::Result<Option<UserRegState>>;

    async fn save_user_reg_authreq(
        &self,
        state_id: &str,
        state: &UserRegState,
    ) -> anyhow::Result<()>;

    async fn take_user_reg_authreq(&self, state_id: &str) -> anyhow::Result<Option<UserRegState>>;

    async fn save_user_login_authreq(&self, state: &UserLoginState) -> anyhow::Result<()>;

    async fn take_user_login_authreq(
        &self,
        state_id: &str,
    ) -> anyhow::Result<Option<UserLoginState>>;
}

#[derive(Clone)]
pub struct RedisOidcStateStore(ConnectionManager);

impl RedisOidcStateStore {
    pub fn new(redis_conn: ConnectionManager) -> Self {
        Self(redis_conn)
    }

    async fn save(&self, key: String, value: String, ttl_secs: u64) -> anyhow::Result<()> {
        let mut conn = self.0.clone();
        conn.set_ex::<_, _, ()>(key, value, ttl_secs).await?;
        Ok(())
    }

    // TODO: GETDEL did not work well here; check the reply type before switching.
    async fn get_then_delete<T: DeserializeOwned>(&self, key: String) -> anyhow::Result<Option<T>> {
        let mut conn = self.0.clone();
        let Some(value) = conn.get::<_, Option<String>>(&key).await? else {
            return Ok(None);
        };
        conn.del::<_, ()>(&key).await?;
        Ok(Some(serde_json::from_str(&value)?))
    }
}

#[async_trait::async_trait]
impl OidcStateStore for RedisOidcStateStore {
    async fn save_user_reg_state(
        &self,
        state_id: &str,
        state: &UserRegState,
    ) -> anyhow::Result<()> {
        self.save(
            user_reg_oauth2_state_key(state_id),
            serde_json::to_string(state)?,
            USER_REG_STATE_TTL_SECS,
        )
        .await
    }

    async fn take_user_reg_state(&self, state_id: &str) -> anyhow::Result<Option<UserRegState>> {
        let mut conn = self.0.clone();
        let Some(value) = conn
            .get_del::<_, Option<String>>(user_reg_oauth2_state_key(state_id))
            .await?
        else {
            return Ok(None);
        };
        Ok(Some(serde_json::from_str(&value)?))
    }

    async fn save_user_reg_authreq(
        &self,
        state_id: &str,
        state: &UserRegState,
    ) -> anyhow::Result<()> {
        self.save(
            user_reg_oauth2_authreq_key(state_id),
            serde_json::to_string(state)?,
            AUTHREQ_TTL_SECS,
        )
        .await
    }

    async fn take_user_reg_authreq(&self, state_id: &str) -> anyhow::Result<Option<UserRegState>> {
        self.get_then_delete(user_reg_oauth2_authreq_key(state_id))
            .await
    }

    async fn save_user_login_authreq(&self, state: &UserLoginState) -> anyhow::Result<()> {
        self.save(
            user_login_oauth2_authreq_key(&state.user_login_state_id),
            serde_json::to_string(state)?,
            AUTHREQ_TTL_SECS,
        )
        .await
    }

    async fn take_user_login_authreq(
        &self,
        state_id: &str,
    ) -> anyhow::Result<Option<UserLoginState>> {
        self.get_then_delete(user_login_oauth2_authreq_key(state_id))
            .await
    }
}
