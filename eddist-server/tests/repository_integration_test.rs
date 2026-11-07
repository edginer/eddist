//! Repository behavior and transaction regression tests against disposable MySQL.
use chrono::{NaiveDate, NaiveDateTime, Timelike};
use eddist::repositories::{
    bbs_repository::{
        AuthedTokenRepository, BbsRepositoryImpl, BoardRepository, CreateThreadError,
        ResponseRepository, ThreadRepository,
    },
    captcha_config_repository::{CaptchaConfigRepository, CaptchaConfigRepositoryImpl},
    idp_repository::{IdpRepository, IdpRepositoryImpl},
    notice_repository::{NoticeRepository, NoticeRepositoryImpl},
    server_settings_repository::{ServerSettingsRepository, ServerSettingsRepositoryImpl},
    stats_repository::{StatsRepository, StatsRepositoryImpl},
    terms_repository::{TermsRepository, TermsRepositoryImpl},
    user_repository::{UserRepository, UserRepositoryImpl},
    user_restriction_repository::{UserRestrictionRepository, UserRestrictionRepositoryImpl},
};
use eddist_core::{
    domain::{
        metadent::MetadentType,
        res::ResView,
        user_restriction::{RestrictionRuleType, RestrictionTarget},
    },
    server_settings::ServerSettingKey,
};
use std::time::Duration;
use uuid::Uuid;

#[path = "support/repository.rs"]
mod repository_support;

use repository_support::*;

#[path = "repository/posting.rs"]
mod posting;

#[path = "repository/users.rs"]
mod users;

#[path = "repository/authed_tokens.rs"]
mod authed_tokens;

#[path = "repository/stats.rs"]
mod stats;

#[path = "repository/boards.rs"]
mod boards;

#[path = "repository/notices.rs"]
mod notices;

#[path = "repository/terms.rs"]
mod terms;

#[path = "repository/settings_idps.rs"]
mod settings_idps;

#[path = "repository/restrictions.rs"]
mod restrictions;

#[path = "repository/captcha.rs"]
mod captcha;

#[path = "repository/core_reads.rs"]
mod core_reads;

#[path = "repository/database_connection.rs"]
mod database_connection;
