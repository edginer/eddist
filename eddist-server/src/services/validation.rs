use std::env;

use eddist_core::{domain::cap::calculate_cap_hash, simple_rate_limiter::RateLimiter};
use redis::aio::ConnectionManager;
use tokio::sync::Mutex;

use crate::{
    domain::{
        authed_token::AuthedToken,
        res::{AuthorIdUninitialized, Res},
        service::bbscgi_auth_service::USER_CREATION_RATE_LIMIT,
    },
    error::BbsCgiError,
    repositories::bbs_repository::BbsRepository,
};

use super::server_settings_cache::{ServerSettingKey, get_server_setting_bool};
use crate::domain::service::bbscgi_user_reg_temp_url_service::{
    UserRegTempUrlService, UserRegUrlKind,
};

/// Returns `Some(err)` if `body` is a `!userreg` command and execution should halt,
/// `None` if the command is not triggered.
pub async fn check_userreg(
    body: &str,
    authed_token: &AuthedToken,
    redis_conn: ConnectionManager,
) -> Result<Option<BbsCgiError>, BbsCgiError> {
    if !get_server_setting_bool(ServerSettingKey::EnableIdpLinking).await
        || !body.starts_with("!userreg")
    {
        return Ok(None);
    }

    let rate_limiter = USER_CREATION_RATE_LIMIT
        .get_or_init(|| Mutex::new(RateLimiter::new(5, std::time::Duration::from_secs(60 * 60))));
    {
        let mut rate_limiter = rate_limiter.lock().await;
        if !rate_limiter.check_and_add(&authed_token.token) {
            return Ok(Some(BbsCgiError::TooManyUserCreationAttempt));
        }
    }

    let user_reg_url_svc = UserRegTempUrlService::new(redis_conn);
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
) -> Result<Option<String>, BbsCgiError> {
    if let Some(cap) = res.cap() {
        let hash = calculate_cap_hash(cap.get(), &env::var("TINKER_SECRET").unwrap());
        Ok(repo
            .get_cap_by_board_key(&hash, board_key)
            .await?
            .map(|x| x.name))
    } else {
        Ok(None)
    }
}
