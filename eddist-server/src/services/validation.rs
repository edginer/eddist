use eddist_core::domain::cap::calculate_cap_hash;
use redis::aio::ConnectionManager;

use crate::{
    config::TinkerSecret,
    domain::{
        authed_token::AuthedToken,
        res::{AuthorIdUninitialized, Res},
        service::bbscgi_auth_service::UserCreationRateLimit,
    },
    error::BbsCgiError,
    repositories::bbs_repository::BbsRepository,
};

use super::server_settings_cache::{ServerSettingKey, ServerSettingsSnapshot};
use crate::domain::service::bbscgi_user_reg_temp_url_service::{
    UserRegTempUrlService, UserRegUrlKind,
};

/// Returns `Some(err)` if `body` is a `!userreg` command and execution should halt,
/// `None` if the command is not triggered.
pub async fn check_userreg(
    body: &str,
    authed_token: &AuthedToken,
    redis_conn: ConnectionManager,
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

    let user_reg_url_svc = UserRegTempUrlService::new(redis_conn, base_url);
    let err = match user_reg_url_svc
        .create_userreg_temp_url(authed_token)
        .await?
    {
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
