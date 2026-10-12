use std::sync::Arc;

use crate::{error::BbsCgiError, stores::auth_state_store::AuthStateStore};

pub const EMAIL_AUTH_PROHIBITED_USER_AGENTS: &[&str] = &[
    "2chMate",
    "mae2c",
    "Geschar",
    "twinkle",
    "Ciisaa",
    "Mozilla/5.0",
];

#[derive(Clone)]
pub struct EmailAuthRestrictionService {
    auth_state: Arc<dyn AuthStateStore>,
}

impl EmailAuthRestrictionService {
    pub fn new(auth_state: Arc<dyn AuthStateStore>) -> Self {
        Self { auth_state }
    }

    /// Check and enforce email auth restriction for prohibited user agents
    /// Returns Ok(()) if request should be allowed, Err(BbsCgiError) if blocked
    pub async fn check_and_enforce_restriction(
        &self,
        is_email_authed: bool,
        user_agent: &str,
        authed_token_id: &str,
        ip_addr: &str,
    ) -> Result<(), BbsCgiError> {
        if !is_email_authed
            || !EMAIL_AUTH_PROHIBITED_USER_AGENTS
                .iter()
                .any(|blocked| user_agent.contains(blocked))
        {
            return Ok(());
        }

        match self.auth_state.has_used_email_auth(authed_token_id).await {
            Ok(true) => {
                // This authed token has already used email auth once - block
                log::warn!(
                    "Blocked creation attempt - email authentication already used once for this token. Token: {authed_token_id}, User-Agent: {user_agent}, IP: {ip_addr}"
                );
                Err(BbsCgiError::EmailAuthenticatedUnsupportedUserAgent)
            }
            Ok(false) => {
                // First time email auth for this token - mark as used and allow
                if let Err(e) = self.auth_state.mark_email_auth_used(authed_token_id).await {
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
