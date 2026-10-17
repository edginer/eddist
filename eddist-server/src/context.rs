use std::sync::Arc;

use crate::{
    config::AppConfig,
    domain::service::{
        bbscgi_auth_service::UserCreationRateLimit, board_info_service::BoardInfoCache,
    },
    services::{
        captcha_config_cache::CaptchaConfigCache, server_settings_cache::ServerSettingsCache,
        stats_counter::StatsCounter, user_restriction_service::UserRestrictionCache,
    },
    tasks::BackgroundTasks,
};

/// Configuration and in-process state owned by one app instance.
#[derive(Clone)]
pub struct AppContext {
    pub config: Arc<AppConfig>,
    pub server_settings: ServerSettingsCache,
    pub captcha_configs: CaptchaConfigCache,
    pub user_restrictions: UserRestrictionCache,
    pub board_info: BoardInfoCache,
    pub stats: StatsCounter,
    pub user_creation_rate_limit: UserCreationRateLimit,
    pub background_tasks: BackgroundTasks,
}
