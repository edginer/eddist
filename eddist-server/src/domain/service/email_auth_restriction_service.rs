use redis::{aio::ConnectionManager, AsyncCommands};

use crate::{
    error::BbsCgiError,
    utils::redis::{email_auth_tinker_used_key, email_auth_used_key},
};

pub const EMAIL_AUTH_PROHIBITED_USER_AGENTS: &[&str] =
    &["2chMate", "mae2c", "Geschar", "twinkle", "Ciisaa"];

#[derive(Clone)]
pub struct EmailAuthRestrictionService {
    redis_conn: ConnectionManager,
}

impl EmailAuthRestrictionService {
    pub fn new(redis_conn: ConnectionManager) -> Self {
        Self { redis_conn }
    }

    /// Check if email auth has been used for this authed token
    async fn has_used_email_auth(&self, authed_token_id: &str) -> anyhow::Result<bool> {
        let key = email_auth_used_key(authed_token_id);
        let mut redis_conn = self.redis_conn.clone();
        let exists = redis_conn.exists::<_, bool>(&key).await?;
        Ok(exists)
    }

    /// Mark email auth as used for this authed token (1 month expiry)
    async fn mark_email_auth_used(&self, authed_token_id: &str) -> anyhow::Result<()> {
        let key = email_auth_used_key(authed_token_id);
        let mut redis_conn = self.redis_conn.clone();

        // Store empty value with 30 days expiry (2,592,000 seconds)
        redis_conn
            .set_ex::<_, _, ()>(&key, "", 60 * 60 * 24 * 30)
            .await?;
        Ok(())
    }

    /// Check if tinker has been enabled for this authed token at least once
    async fn has_enabled_tinker(&self, authed_token_id: &str) -> anyhow::Result<bool> {
        let key = email_auth_tinker_used_key(authed_token_id);
        let mut redis_conn = self.redis_conn.clone();
        let exists = redis_conn.exists::<_, bool>(&key).await?;
        Ok(exists)
    }

    /// Mark that tinker has been enabled for this authed token (1 month expiry)
    async fn mark_tinker_enabled(&self, authed_token_id: &str) -> anyhow::Result<()> {
        let key = email_auth_tinker_used_key(authed_token_id);
        let mut redis_conn = self.redis_conn.clone();
        // Store empty value with 30 days expiration (2,592,000 seconds)
        redis_conn
            .set_ex::<_, _, ()>(&key, "", 60 * 60 * 24 * 30)
            .await?;
        Ok(())
    }

    /// Check and enforce email auth restriction when tinker was previously enabled but is now disabled
    async fn check_and_enforce_tinker_restriction(
        &self,
        authed_token_id: &str,
        is_tinker_active: bool,
    ) -> Result<(), BbsCgiError> {
        match self.has_enabled_tinker(authed_token_id).await {
            Ok(true) => {
                // Tinker has been enabled before
                if !is_tinker_active {
                    // Now tinker is disabled - block
                    log::warn!(
                        "Blocked creation attempt - tinker was enabled before but now disabled. Token: {authed_token_id}"
                    );
                    if std::env::var("EMAIL_AUTH_TINKER_ENFORCE_LOG_ONLY")
                        .unwrap_or("true".to_string())
                        .eq_ignore_ascii_case("true")
                    {
                        log::info!("EMAIL_AUTH_TINKER_ENFORCE_LOG_ONLY is set - allowing despite tinker restriction.");
                        return Ok(());
                    }
                    Err(BbsCgiError::EmailAuthenticatedUnsupportedUserAgent)
                } else {
                    // Tinker still enabled - allow
                    Ok(())
                }
            }
            Ok(false) => {
                // Tinker has never been enabled before - allow and mark if now enabled
                if is_tinker_active {
                    if let Err(e) = self.mark_tinker_enabled(authed_token_id).await {
                        log::error!("Failed to mark tinker as enabled: {e:?}");
                    }
                }
                Ok(())
            }
            Err(e) => {
                log::error!("Failed to check tinker usage: {e:?}");
                // On Redis error, allow to prevent blocking legitimate users
                Ok(())
            }
        }
    }

    /// Check and enforce email auth restriction for prohibited user agents
    /// Returns Ok(()) if request should be allowed, Err(BbsCgiError) if blocked
    pub async fn check_and_enforce_restriction(
        &self,
        is_email_authed: bool,
        user_agent: &str,
        authed_token_id: &str,
        ip_addr: &str,
        is_tinker_active: bool,
    ) -> Result<(), BbsCgiError> {
        if !is_email_authed
            || !EMAIL_AUTH_PROHIBITED_USER_AGENTS
                .iter()
                .any(|blocked| user_agent.contains(blocked))
        {
            return Ok(());
        }

        self.check_and_enforce_tinker_restriction(authed_token_id, is_tinker_active)
            .await?;

        match self.has_used_email_auth(authed_token_id).await {
            Ok(true) => {
                // This authed token has already used email auth once - block
                log::warn!(
                    "Blocked creation attempt - email authentication already used once for this token. Token: {authed_token_id}, User-Agent: {user_agent}, IP: {ip_addr}"
                );
                Err(BbsCgiError::EmailAuthenticatedUnsupportedUserAgent)
            }
            Ok(false) => {
                // First time email auth for this token - mark as used and allow
                if let Err(e) = self.mark_email_auth_used(authed_token_id).await {
                    log::error!("Failed to mark email auth as used: {e:?}");
                }
                log::info!(
                    "Allowed first-time email authentication for prohibited User-Agent. Token: {authed_token_id}, User-Agent: {user_agent}, IP: {ip_addr}"
                );
                Ok(())
            }
            Err(e) => {
                log::error!("Failed to check email auth usage: {e:?}");
                // On Redis error, allow to prevent blocking legitimate users
                Ok(())
            }
        }
    }
}
