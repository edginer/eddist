pub use axum;
pub use chrono;
pub use handlebars::Handlebars;
pub use uuid;

mod shiftjis;
pub mod repositories {
    pub mod bbs_pubsub_repository;
    pub mod bbs_repository;
    pub mod captcha_config_repository;
    pub mod idp_repository;
    pub mod notice_repository;
    pub mod server_settings_repository;
    pub mod stats_repository;
    pub mod terms_repository;
    pub mod user_repository;
    pub mod user_restriction_repository;
}
mod domain {
    pub(crate) mod service {
        pub mod bbscgi_auth_service;
        pub mod bbscgi_user_reg_temp_url_service;
        pub mod board_info_service;
        pub mod email_auth_restriction_service;
        pub mod oidc_client_service;
    }
    pub(crate) mod authed_token;
    pub(crate) mod captcha_like;
    pub(crate) mod metadent;
    pub(crate) mod ng_word;
    pub(crate) mod posting_policy;
    pub(crate) mod res;
    pub(crate) mod res_core;
    pub(crate) mod thread;
    pub(crate) mod thread_list;
    pub(crate) mod thread_res_list;
    pub(crate) mod user;
    pub(crate) mod utils;
}
pub mod config;
pub mod context;
pub mod database;
mod error;
pub mod middleware;
pub mod services;
pub(crate) mod stores;
pub mod tasks;
mod template;
pub(crate) mod external {
    pub mod captcha_like_client;
    pub mod oidc_client;
}
pub(crate) mod utils;
mod routes {
    pub mod auth_code;
    pub mod bbs_cgi;
    pub mod dat_routing;
    pub mod ng_id;
    pub mod notice;
    pub mod re_auth;
    pub mod safe_mode;
    pub mod stats;
    pub mod subject_list;
    pub mod terms;
    pub mod user;
}

pub mod app;

pub use app::AppState;
