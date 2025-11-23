use std::path::Path;

pub use axum;
use base64::Engine;
pub use chrono;
pub use handlebars::Handlebars;
pub use sqlx;
pub use uuid;

mod shiftjis;
mod repositories {
    pub(crate) mod bbs_pubsub_repository;
    pub(crate) mod bbs_repository;
    pub(crate) mod idp_repository;
    pub(crate) mod notice_repository;
    pub(crate) mod user_repository;
    pub(crate) mod user_restriction_repository;
}
mod domain {
    pub(crate) mod service {
        pub mod bbscgi_auth_service;
        pub mod bbscgi_user_reg_temp_url_service;
        pub mod board_info_service;
        pub mod email_auth_restriction_service;
        pub mod ng_word_reading_service;
        pub mod oidc_client_service;
        pub mod res_creation_span_management_service;
    }
    pub(crate) mod authed_token;
    pub(crate) mod captcha_like;
    pub(crate) mod metadent;
    pub(crate) mod ng_word;
    pub(crate) mod res;
    pub(crate) mod res_core;
    pub(crate) mod thread;
    pub(crate) mod thread_list;
    pub(crate) mod thread_res_list;
    pub(crate) mod user;
    pub(crate) mod utils;
}
mod error;
mod middleware;
mod services;
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
    pub mod notice;
    pub mod statics;
    pub mod subject_list;
    pub mod user;
}

pub mod app;

pub use app::AppState;
use uuid::Uuid;

use crate::repositories::notice_repository::NoticeRepositoryImpl;
pub use crate::services::user_restriction_service::start_cache_refresh_task;
pub use crate::template::load_template_engine;

// Test app factory for integration tests
pub fn create_test_app(
    pool: sqlx::MySqlPool,
    redis_conn: redis::aio::ConnectionManager,
) -> axum::Router {
    use crate::repositories::{
        bbs_pubsub_repository::{RedisCreationEventRepository, RedisPubRepository},
        bbs_repository::BbsRepositoryImpl,
        idp_repository::IdpRepositoryImpl,
        user_repository::UserRepositoryImpl,
        user_restriction_repository::UserRestrictionRepositoryImpl,
    };
    use crate::services::AppServiceContainer;
    use tower_http::services::{ServeDir, ServeFile};

    // Create test S3 bucket (stub for testing)
    let bucket = s3::Bucket::new(
        "test-bucket",
        s3::Region::R2 {
            account_id: "test".to_string(),
        },
        s3::creds::Credentials::new(Some("test"), Some("test"), None, None, None).unwrap(),
    )
    .unwrap();

    let user_restriction_repo = UserRestrictionRepositoryImpl::new(pool.clone());
    let pub_repo = RedisPubRepository::new(redis_conn.clone());
    let event_repo = RedisCreationEventRepository::new(redis_conn.clone());
    let notice_repo = NoticeRepositoryImpl::new(pool.clone());

    // Check existence of index.html for ServeFile
    let path = Path::new("client/dist/index.html");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let _ = std::fs::metadata(path).map_err(|_| std::fs::File::create(path).unwrap());

    let app_state = AppState {
        services: AppServiceContainer::new(
            BbsRepositoryImpl::new(pool.clone()),
            UserRepositoryImpl::new(pool.clone()),
            IdpRepositoryImpl::new(pool.clone()),
            user_restriction_repo,
            redis_conn.clone(),
            pub_repo,
            event_repo,
            *bucket,
        ),
        notice_repo,
        tinker_secret: base64::engine::general_purpose::STANDARD
            .encode(Uuid::new_v4().as_bytes())
            .to_string(),
        captcha_like_configs: vec![],
        template_engine: load_template_engine("client/dist/index.html"),
    };

    // Create minimal serve directory for tests
    let serve_file = ServeFile::new("client/dist/index.html");
    let serve_dir = ServeDir::new("client/dist").not_found_service(serve_file.clone());
    let serve_dir_inner = serve_dir.clone();

    // Use the actual create_app from app module
    app::create_app(app_state, redis_conn, serve_dir, serve_dir_inner)
}

// Simple test helper that doesn't require exposing private types
pub mod test_helpers {
    use chrono::Utc;
    use sqlx::{MySqlPool, Row};
    use uuid::Uuid;

    /// Create a test board directly in the database
    pub async fn create_test_board(pool: &MySqlPool, board_key: &str, name: &str) -> Uuid {
        let board_id = Uuid::now_v7();
        sqlx::query(
            r#"
            INSERT INTO boards (id, name, board_key, default_name)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(board_id)
        .bind(name)
        .bind(board_key)
        .bind("名無しさん")
        .execute(pool)
        .await
        .expect("Failed to create test board");

        sqlx::query(
            r#"
            INSERT INTO boards_info (id, local_rules, created_at, updated_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(board_id)
        .bind("テストルール")
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(pool)
        .await
        .expect("Failed to create board info");

        board_id
    }

    /// Create a test authed token
    pub async fn create_test_authed_token(
        pool: &MySqlPool,
        origin_ip: &str,
        auth_code: &str,
    ) -> (Uuid, String) {
        let token_id = Uuid::now_v7();
        let token = format!("test-token-{}", Uuid::new_v4());

        sqlx::query(
            r#"
            INSERT INTO authed_tokens
            (id, token, origin_ip, reduced_origin_ip, writing_ua, authed_ua, auth_code, created_at, authed_at, validity, last_wrote_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(token_id)
        .bind(&token)
        .bind(origin_ip)
        .bind(origin_ip)
        .bind("Test User Agent")
        .bind("Test User Agent")
        .bind(auth_code)
        .bind(Utc::now())
        .bind(Utc::now())
        .bind(true)
        .bind(Option::<chrono::DateTime<Utc>>::None)
        .execute(pool)
        .await
        .expect("Failed to create authed token");

        (token_id, token)
    }

    /// Create a test thread
    pub async fn create_test_thread(
        pool: &MySqlPool,
        board_id: Uuid,
        thread_number: i64,
        title: &str,
        authed_token_id: Uuid,
    ) -> Uuid {
        let thread_id = Uuid::now_v7();

        sqlx::query(
            r#"
            INSERT INTO threads
            (id, board_id, thread_number, last_modified_at, sage_last_modified_at,
             title, authed_token_id, metadent, response_count, no_pool, active, archived, archive_converted)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(thread_id)
        .bind(board_id)
        .bind(thread_number)
        .bind(Utc::now())
        .bind(Utc::now())
        .bind(title)
        .bind(authed_token_id)
        .bind("")
        .bind(0)
        .bind(false)
        .bind(true)
        .bind(false)
        .bind(false)
        .execute(pool)
        .await
        .expect("Failed to create thread");

        thread_id
    }

    /// Create a test response
    pub async fn create_test_response(
        pool: &MySqlPool,
        board_id: Uuid,
        thread_id: Uuid,
        authed_token_id: Uuid,
        res_order: i32,
        body: &str,
    ) -> Uuid {
        let res_id = Uuid::now_v7();
        let client_info = serde_json::json!({
            "user_agent": "Test UA",
            "ip_addr": "127.0.0.1",
            "asn_num": 0,
            "tinker": null
        });

        sqlx::query(
            r#"
            INSERT INTO responses
            (id, author_name, mail, body, created_at, author_id, ip_addr,
             authed_token_id, board_id, thread_id, is_abone, res_order, client_info)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(res_id)
        .bind("名無しさん")
        .bind("")
        .bind(body)
        .bind(Utc::now())
        .bind("test-id")
        .bind("127.0.0.1")
        .bind(authed_token_id)
        .bind(board_id)
        .bind(thread_id)
        .bind(false)
        .bind(res_order)
        .bind(client_info.to_string())
        .execute(pool)
        .await
        .expect("Failed to create response");

        // Update thread response count
        sqlx::query(
            r#"
            UPDATE threads SET response_count = response_count + 1, last_modified_at = ?
            WHERE id = ?
            "#,
        )
        .bind(Utc::now())
        .bind(thread_id)
        .execute(pool)
        .await
        .expect("Failed to update thread response count");

        res_id
    }

    /// Get thread count for a board
    pub async fn get_thread_count(pool: &MySqlPool, board_id: Uuid) -> i64 {
        sqlx::query("SELECT COUNT(*) as count FROM threads WHERE board_id = ?")
            .bind(board_id)
            .fetch_one(pool)
            .await
            .expect("Failed to count threads")
            .get("count")
    }

    /// Get response count for a thread
    pub async fn get_response_count(pool: &MySqlPool, thread_id: Uuid) -> i64 {
        sqlx::query("SELECT COUNT(*) as count FROM responses WHERE thread_id = ?")
            .bind(thread_id)
            .fetch_one(pool)
            .await
            .expect("Failed to count responses")
            .get("count")
    }

    /// Encode form data to Shift-JIS URL encoding
    pub fn encode_sjis_form(params: &[(&str, &str)]) -> String {
        use encoding_rs::SHIFT_JIS;

        params
            .iter()
            .map(|(key, value)| {
                let (encoded_value, _, _) = SHIFT_JIS.encode(value);
                let url_encoded: String = encoded_value
                    .iter()
                    .map(|&b| {
                        if b.is_ascii_alphanumeric()
                            || b == b'-'
                            || b == b'_'
                            || b == b'.'
                            || b == b'~'
                        {
                            (b as char).to_string()
                        } else {
                            format!("%{:02X}", b)
                        }
                    })
                    .collect();
                format!("{key}={url_encoded}")
            })
            .collect::<Vec<_>>()
            .join("&")
    }

    /// Decode Shift-JIS response
    pub fn decode_sjis(bytes: &[u8]) -> String {
        use encoding_rs::SHIFT_JIS;
        let (decoded, _, _) = SHIFT_JIS.decode(bytes);
        decoded.into_owned()
    }
}
