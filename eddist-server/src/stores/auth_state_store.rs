use eddist_core::redis_keys::{
    authed_token_suspended_key, email_auth_used_key, reauth_lock_key, reauth_temp_key,
    user_reg_temp_url_register_key,
};
use rand::RngExt;
use redis::{AsyncCommands, aio::ConnectionManager};
use uuid::Uuid;

use crate::domain::{
    service::bbscgi_user_reg_temp_url_service::{USER_REG_TEMP_URL_LEN, generate_random_string},
    user::user_reg_state::TempUrlRegistrationRecord,
};

const REAUTH_CODE_TTL_SECS: u64 = 60 * 5;
const USER_REG_TEMP_URL_TTL_SECS: u64 = 60 * 3;
const EMAIL_AUTH_USED_TTL_SECS: u64 = 60 * 60 * 24 * 30;

#[async_trait::async_trait]
pub trait AuthStateStore: Send + Sync {
    /// A Redis failure counts as not suspended.
    async fn is_token_suspended(&self, authed_token_id: Uuid) -> bool;

    /// Reuses the token's pending code (refreshing its TTL) so repeated posts keep at most
    /// two keys per token. Redis failures are ignored and still yield a code.
    async fn issue_reauth_code(&self, authed_token_id: Uuid) -> String;

    /// Single use, and frees the token to be issued a new code. A Redis failure or a
    /// malformed stored value counts as an unknown code.
    async fn consume_reauth_code(&self, code: &str) -> Option<Uuid>;

    /// Returns the URL path segment under `/user/register/`.
    async fn issue_user_reg_temp_url(
        &self,
        record: &TempUrlRegistrationRecord,
    ) -> anyhow::Result<String>;

    async fn consume_user_reg_temp_url(
        &self,
        path: &str,
    ) -> anyhow::Result<Option<TempUrlRegistrationRecord>>;

    async fn has_used_email_auth(&self, token: &str) -> anyhow::Result<bool>;

    async fn mark_email_auth_used(&self, token: &str) -> anyhow::Result<()>;
}

#[derive(Clone)]
pub struct RedisAuthStateStore(ConnectionManager);

impl RedisAuthStateStore {
    pub fn new(redis_conn: ConnectionManager) -> Self {
        Self(redis_conn)
    }
}

#[async_trait::async_trait]
impl AuthStateStore for RedisAuthStateStore {
    async fn is_token_suspended(&self, authed_token_id: Uuid) -> bool {
        let mut conn = self.0.clone();
        conn.exists::<_, bool>(authed_token_suspended_key(&authed_token_id.to_string()))
            .await
            .unwrap_or(false)
    }

    async fn issue_reauth_code(&self, authed_token_id: Uuid) -> String {
        let mut conn = self.0.clone();
        let token_id_str = authed_token_id.to_string();
        let lock_key = reauth_lock_key(&token_id_str);
        let existing_code: Option<String> = conn.get(&lock_key).await.unwrap_or(None);
        match existing_code {
            Some(code) => {
                let _ = conn
                    .expire::<_, ()>(&lock_key, REAUTH_CODE_TTL_SECS as i64)
                    .await;
                let _ = conn
                    .expire::<_, ()>(&reauth_temp_key(&code), REAUTH_CODE_TTL_SECS as i64)
                    .await;
                code
            }
            None => {
                let code = gen_reauth_temp_key();
                let _ = conn
                    .set_ex::<_, _, ()>(
                        &reauth_temp_key(&code),
                        &token_id_str,
                        REAUTH_CODE_TTL_SECS,
                    )
                    .await;
                let _ = conn
                    .set_ex::<_, _, ()>(&lock_key, &code, REAUTH_CODE_TTL_SECS)
                    .await;
                code
            }
        }
    }

    async fn consume_reauth_code(&self, code: &str) -> Option<Uuid> {
        let mut conn = self.0.clone();
        let token_id_str: Option<String> =
            conn.get_del(reauth_temp_key(code)).await.unwrap_or(None);
        let token_id = Uuid::parse_str(&token_id_str?).ok()?;
        let _ = conn
            .del::<_, ()>(reauth_lock_key(&token_id.to_string()))
            .await;
        Some(token_id)
    }

    async fn issue_user_reg_temp_url(
        &self,
        record: &TempUrlRegistrationRecord,
    ) -> anyhow::Result<String> {
        let mut conn = self.0.clone();

        let mut path = generate_random_string(USER_REG_TEMP_URL_LEN);
        if conn
            .exists::<_, bool>(user_reg_temp_url_register_key(&path))
            .await?
        {
            // Retries once; a collision between EXISTS and SET is not handled.
            path = generate_random_string(USER_REG_TEMP_URL_LEN);
            if conn
                .exists::<_, bool>(user_reg_temp_url_register_key(&path))
                .await?
            {
                return Err(anyhow::anyhow!("Failed to generate temp_url_query"));
            }
        }

        conn.set_ex::<_, _, ()>(
            user_reg_temp_url_register_key(&path),
            serde_json::to_string(record)?,
            USER_REG_TEMP_URL_TTL_SECS,
        )
        .await?;

        Ok(path)
    }

    async fn consume_user_reg_temp_url(
        &self,
        path: &str,
    ) -> anyhow::Result<Option<TempUrlRegistrationRecord>> {
        let mut conn = self.0.clone();
        let Some(record) = conn
            .get_del::<_, Option<String>>(user_reg_temp_url_register_key(path))
            .await?
        else {
            return Ok(None);
        };
        Ok(Some(serde_json::from_str(&record)?))
    }

    async fn has_used_email_auth(&self, token: &str) -> anyhow::Result<bool> {
        let mut conn = self.0.clone();
        Ok(conn.exists::<_, bool>(email_auth_used_key(token)).await?)
    }

    async fn mark_email_auth_used(&self, token: &str) -> anyhow::Result<()> {
        let mut conn = self.0.clone();
        conn.set_ex::<_, _, ()>(email_auth_used_key(token), "", EMAIL_AUTH_USED_TTL_SECS)
            .await?;
        Ok(())
    }
}

/// 8 Crockford Base32 characters (no I/L/O/U): ~1 trillion keys, enough for a 5-minute TTL.
fn gen_reauth_temp_key() -> String {
    const CHARS: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    let mut rng = rand::rng();
    (0..8)
        .map(|_| CHARS[rng.random_range(0..CHARS.len())] as char)
        .collect()
}
