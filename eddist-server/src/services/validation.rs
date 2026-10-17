use eddist_core::domain::cap::calculate_cap_hash;

use crate::{
    config::TinkerSecret,
    domain::{
        authed_token::AuthedToken,
        res::{AuthorIdUninitialized, Res},
        service::bbscgi_auth_service::UserCreationRateLimit,
    },
    error::BbsCgiError,
    repositories::bbs_repository::BbsRepository,
    stores::auth_state_store::AuthStateStore,
};

use super::server_settings_cache::{ServerSettingKey, ServerSettingsSnapshot};
use crate::domain::service::bbscgi_user_reg_temp_url_service::{
    UserRegUrlKind, create_userreg_temp_url,
};

/// Returns `Some(err)` if `body` is a `!userreg` command and execution should halt,
/// `None` if the command is not triggered.
pub async fn check_userreg(
    body: &str,
    authed_token: &AuthedToken,
    auth_state: &dyn AuthStateStore,
    settings: &ServerSettingsSnapshot,
    base_url: &str,
    rate_limit: &UserCreationRateLimit,
) -> Result<Option<BbsCgiError>, BbsCgiError> {
    if !settings.is_enabled(ServerSettingKey::EnableIdpLinking) || !body.starts_with("!userreg") {
        return Ok(None);
    }

    if !rate_limit.check_and_add(&authed_token.token).await {
        return Ok(Some(BbsCgiError::TooManyUserCreationAttempt));
    }

    let err = match create_userreg_temp_url(auth_state, base_url, authed_token).await? {
        UserRegUrlKind::Registered => BbsCgiError::UserAlreadyRegistered,
        UserRegUrlKind::NotRegistered(url) => BbsCgiError::UserRegTempUrl { url },
    };
    Ok(Some(err))
}

/// Resolves the cap display name for a response, if a valid cap hash is present.
pub async fn resolve_cap_name(
    repo: &impl BbsRepository,
    res: &Res<AuthorIdUninitialized>,
    board_key: &str,
    tinker_secret: &TinkerSecret,
) -> Result<Option<String>, BbsCgiError> {
    if let Some(cap) = res.cap() {
        let hash = calculate_cap_hash(cap.get(), tinker_secret.raw());
        Ok(repo
            .get_cap_by_board_key(&hash, board_key)
            .await?
            .map(|x| x.name))
    } else {
        Ok(None)
    }
}
