use std::collections::HashMap;

use crate::entity::{
    archived_response, archived_thread, authed_token, board_cap, board_ng_word, idp, response,
    thread, user, user_authed_token, user_idp_binding,
};
use crate::models::{
    CreateBoardInput, CreateCaptchaConfigInput, CreateIdpInput, EditBoardInput, HttpMethod,
    RequestFormat, UpdateCaptchaConfigInput, UpdateIdpInput,
};
use crate::repository::{
    admin_board_repository::{AdminBoardRepository, AdminBoardRepositoryImpl},
    admin_response_repository::{AdminResponseRepository, AdminResponseRepositoryImpl},
    admin_thread_repository::{AdminThreadRepository, AdminThreadRepositoryImpl},
    admin_user_repository::{AdminUserRepository, AdminUserRepositoryImpl},
    authed_token_repository::{AuthedTokenRepository, AuthedTokenRepositoryImpl},
    cap_repository::{CapRepository, CapRepositoryImpl},
    captcha_config_repository::{CaptchaConfigRepository, CaptchaConfigRepositoryImpl},
    idp_repository::{IdpAdminRepository, IdpAdminRepositoryImpl},
    ngword_repository::{NgWordRepository, NgWordRepositoryImpl},
    notice_repository::{
        CreateNoticeInput, NoticeRepository, NoticeRepositoryImpl, UpdateNoticeInput,
    },
    server_settings_repository::{ServerSettingsRepository, ServerSettingsRepositoryImpl},
    terms_repository::{TermsRepository, TermsRepositoryImpl, UpdateTermsInput},
    user_restriction_repository::UserRestrictionRepository,
};
use chrono::{Duration, Utc};
use eddist_core::domain::user_restriction::{
    CreateUserRestrictionRuleInput, RestrictionRuleType, UpdateUserRestrictionRuleInput,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, Database, DatabaseConnection, EntityTrait,
    QueryFilter,
};
use testcontainers::{ContainerAsync, ImageExt, core::IntoContainerPort, runners::AsyncRunner};
use testcontainers_modules::mysql::Mysql;
use uuid::Uuid;

struct MysqlTestDatabase {
    _container: ContainerAsync<Mysql>,
    db: DatabaseConnection,
}

async fn setup_database() -> anyhow::Result<MysqlTestDatabase> {
    let container = Mysql::default().with_tag("8.0").start().await?;
    let port = container.get_host_port_ipv4(3306.tcp()).await?;
    let database_url = format!("mysql://root@127.0.0.1:{port}/test");

    let migration_pool = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    sqlx::migrate!("../migrations").run(&migration_pool).await?;
    migration_pool.close().await;

    let mut options = sea_orm::ConnectOptions::new(database_url);
    options.sqlx_logging(false);
    let db = Database::connect(options).await?;

    Ok(MysqlTestDatabase {
        _container: container,
        db,
    })
}

fn empty_edit_board_input() -> EditBoardInput {
    EditBoardInput {
        name: None,
        default_name: None,
        local_rule: None,
        base_thread_creation_span_sec: None,
        base_response_creation_span_sec: None,
        max_thread_name_byte_length: None,
        max_author_name_byte_length: None,
        max_email_byte_length: None,
        max_response_body_byte_length: None,
        max_response_body_lines: None,
        threads_archive_cron: None,
        threads_archive_trigger_thread_count: None,
        read_only: None,
        force_metadent_type: None,
        enable_1001_message: None,
        custom_1001_message: None,
    }
}

fn assert_not_found(error: anyhow::Error, expected: &str) {
    let service_error = error
        .chain()
        .find_map(|cause| cause.downcast_ref::<crate::error::ServiceError>())
        .unwrap_or_else(|| panic!("expected ServiceError::NotFound, got {error:?}"));
    assert!(
        matches!(
            service_error,
            crate::error::ServiceError::NotFound(message) if message.contains(expected)
        ),
        "expected NotFound containing {expected:?}, got {service_error:?}"
    );
}

fn create_board_input(board_key: &str, name: &str) -> CreateBoardInput {
    CreateBoardInput {
        name: name.to_string(),
        board_key: board_key.to_string(),
        default_name: "名無しさん".to_string(),
        local_rule: "利用規約に従う".to_string(),
        base_thread_creation_span_sec: None,
        base_response_creation_span_sec: None,
        max_thread_name_byte_length: None,
        max_author_name_byte_length: None,
        max_email_byte_length: None,
        max_response_body_byte_length: None,
        max_response_body_lines: None,
        threads_archive_cron: None,
        threads_archive_trigger_thread_count: None,
        force_metadent_type: None,
    }
}

async fn insert_authed_token(db: &DatabaseConnection, id: Uuid) -> anyhow::Result<()> {
    let now = crate::db_time::now();
    authed_token::ActiveModel {
        id: Set(id),
        token: Set(format!("integration-token-{id}")),
        origin_ip: Set("127.0.0.1".to_string()),
        reduced_origin_ip: Set("127.0.0.1".to_string()),
        writing_ua: Set("integration-test".to_string()),
        auth_code: Set("itest".to_string()),
        created_at: Set(now),
        validity: Set(true),
        asn_num: Set(64512),
        require_user_registration: Set(false),
        require_reauth: Set(false),
        author_id_seed: Set(vec![0; 64]),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(())
}

async fn insert_thread(
    db: &DatabaseConnection,
    id: Uuid,
    board_id: Uuid,
    token_id: Uuid,
    thread_number: i64,
    last_modified_at: chrono::NaiveDateTime,
) -> anyhow::Result<()> {
    thread::ActiveModel {
        id: Set(id),
        board_id: Set(board_id),
        thread_number: Set(thread_number),
        last_modified_at: Set(last_modified_at),
        sage_last_modified_at: Set(last_modified_at),
        title: Set(format!("thread-{thread_number}")),
        authed_token_id: Set(token_id),
        metadent: Set("metadent".to_string()),
        response_count: Set(1),
        no_pool: Set(false),
        active: Set(true),
        archived: Set(false),
        archive_converted: Set(false),
    }
    .insert(db)
    .await?;
    Ok(())
}

async fn insert_response(
    db: &DatabaseConnection,
    id: Uuid,
    board_id: Uuid,
    thread_id: Uuid,
    token_id: Uuid,
    created_at: chrono::NaiveDateTime,
) -> anyhow::Result<()> {
    response::ActiveModel {
        id: Set(id),
        author_name: Set("投稿者".to_string()),
        mail: Set("sage".to_string()),
        body: Set("本文".to_string()),
        created_at: Set(created_at),
        author_id: Set("author".to_string()),
        ip_addr: Set("127.0.0.1".to_string()),
        authed_token_id: Set(token_id),
        board_id: Set(board_id),
        thread_id: Set(thread_id),
        is_abone: Set(false),
        res_order: Set(1),
        client_info: Set(serde_json::json!({
            "user_agent": "integration-test",
            "asn_num": 64512,
            "ip_addr": "127.0.0.1",
            "tinker": null
        })),
        is_abone_keep_id: Set(false),
    }
    .insert(db)
    .await?;
    Ok(())
}

async fn insert_archived_thread_and_response(
    db: &DatabaseConnection,
    board_id: Uuid,
    token_id: Uuid,
) -> anyhow::Result<(Uuid, Uuid)> {
    let thread_id = Uuid::now_v7();
    let response_id = Uuid::now_v7();
    let timestamp = crate::db_time::now();

    archived_thread::Entity::insert(archived_thread::ActiveModel {
        id: Set(thread_id),
        board_id: Set(board_id),
        thread_number: Set(9001),
        last_modified_at: Set(timestamp),
        sage_last_modified_at: Set(timestamp),
        title: Set("archived-thread".to_string()),
        authed_token_id: Set(token_id),
        metadent: Set("metadent".to_string()),
        response_count: Set(1),
        no_pool: Set(false),
        active: Set(false),
        archived: Set(true),
    })
    .exec(db)
    .await?;

    archived_response::Entity::insert(archived_response::ActiveModel {
        id: Set(response_id),
        author_name: Set("投稿者".to_string()),
        mail: Set(String::new()),
        body: Set("過去の本文".to_string()),
        created_at: Set(timestamp),
        author_id: Set("author".to_string()),
        ip_addr: Set("127.0.0.1".to_string()),
        authed_token_id: Set(token_id),
        board_id: Set(board_id),
        thread_id: Set(thread_id),
        is_abone: Set(false),
        res_order: Set(1),
        client_info: Set(serde_json::json!({
            "user_agent": "integration-test",
            "asn_num": 64512,
            "ip_addr": "127.0.0.1",
            "tinker": null
        })),
    })
    .exec(db)
    .await?;

    Ok((thread_id, response_id))
}

#[tokio::test]
async fn seaorm_admin_crud_round_trips_against_mysql() -> anyhow::Result<()> {
    unsafe {
        std::env::set_var(
            "TINKER_SECRET",
            "integration-test-secret-that-is-long-enough",
        );
    }

    let test_database = setup_database().await?;
    let db = &test_database.db;

    let board_repository = AdminBoardRepositoryImpl::new(db.clone());
    let board = board_repository
        .create_board(create_board_input("orm-it", "ORM test board"))
        .await?;
    let second_board = board_repository
        .create_board(create_board_input("orm-it-2", "ORM second board"))
        .await?;
    let board_info = board_repository.get_board_info(board.id).await?;
    assert_eq!(board_info.base_thread_creation_span_sec, 120);
    assert_eq!(board_info.max_response_body_lines, 32);

    let edited_board = board_repository
        .edit_board(
            "orm-it",
            EditBoardInput {
                name: Some("Edited board".to_string()),
                default_name: None,
                local_rule: None,
                base_thread_creation_span_sec: None,
                base_response_creation_span_sec: None,
                max_thread_name_byte_length: None,
                max_author_name_byte_length: None,
                max_email_byte_length: None,
                max_response_body_byte_length: None,
                max_response_body_lines: None,
                threads_archive_cron: Some(String::new()),
                threads_archive_trigger_thread_count: None,
                read_only: None,
                force_metadent_type: Some(String::new()),
                enable_1001_message: Some(false),
                custom_1001_message: Some(String::new()),
            },
        )
        .await?;
    assert_eq!(edited_board.name, "Edited board");
    assert_eq!(edited_board.default_name, board.default_name);
    let edited_info = board_repository.get_board_info(board.id).await?;
    assert_eq!(edited_info.base_thread_creation_span_sec, 120);
    assert_eq!(edited_info.threads_archive_cron, None);
    assert_eq!(edited_info.force_metadent_type, None);
    assert!(!edited_info.enable_1001_message);
    assert_eq!(edited_info.custom_1001_message, None);

    let token_id = Uuid::now_v7();
    insert_authed_token(db, token_id).await?;
    let thread_id = Uuid::now_v7();
    let second_thread_id = Uuid::now_v7();
    let now = crate::db_time::now();
    insert_thread(db, thread_id, board.id, token_id, 1001, now).await?;
    insert_thread(
        db,
        second_thread_id,
        board.id,
        token_id,
        1002,
        now + Duration::seconds(1),
    )
    .await?;
    // Boards with no threads must still be returned, with a zero count: the thread counts come
    // from a GROUP BY that omits them entirely.
    let all_boards = board_repository.get_boards_by_key(None).await?;
    let counts = all_boards
        .iter()
        .map(|board| (board.board_key.as_str(), board.thread_count))
        .collect::<HashMap<_, _>>();
    assert_eq!(counts.get("orm-it").copied(), Some(2));
    assert_eq!(counts.get("orm-it-2").copied(), Some(0));

    let filtered_boards = board_repository
        .get_boards_by_key(Some(vec!["orm-it".to_string()]))
        .await?;
    assert_eq!(filtered_boards.len(), 1);
    assert_eq!(filtered_boards[0].thread_count, 2);

    let response_id = Uuid::now_v7();
    insert_response(db, response_id, board.id, thread_id, token_id, now).await?;
    let (_, archived_response_id) =
        insert_archived_thread_and_response(db, board.id, token_id).await?;

    let response_repository = AdminResponseRepositoryImpl::new(db.clone());
    let responses = response_repository
        .get_reses_by_thread_id("orm-it", 1001)
        .await?;
    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].id, response_id);
    let updated_response = response_repository
        .update_res(
            response_id,
            Some("編集者".to_string()),
            None,
            Some("編集後の本文".to_string()),
            Some(true),
            Some(true),
        )
        .await?;
    assert_eq!(updated_response.author_name.as_deref(), Some("編集者"));
    assert!(updated_response.is_abone);
    assert!(updated_response.is_abone_keep_id);
    let (loaded_response, default_name, board_key, thread_number, title) =
        response_repository.get_res(response_id).await?;
    assert_eq!(loaded_response.id, response_id);
    assert_eq!(default_name, board.default_name);
    assert_eq!(board_key, "orm-it");
    assert_eq!(thread_number, 1001);
    assert_eq!(title.as_deref(), Some("thread-1001"));
    let archived_responses = response_repository
        .get_archived_reses_by_thread_id("orm-it", 9001)
        .await?;
    assert_eq!(archived_responses.len(), 1);
    assert_eq!(archived_responses[0].id, archived_response_id);
    assert!(!archived_responses[0].is_abone_keep_id);

    let thread_repository = AdminThreadRepositoryImpl::new(db.clone());
    let threads = thread_repository
        .get_threads_by_thread_id("orm-it", Some(vec![1001, 1002]))
        .await?;
    assert_eq!(threads.len(), 2);
    thread_repository.archive_threads("orm-it", &[1001]).await?;
    let archived_current = thread::Entity::find_by_id(thread_id)
        .one(db)
        .await?
        .unwrap();
    assert!(archived_current.archived);
    assert!(!archived_current.active);
    thread_repository.compact_threads("orm-it", 0).await?;
    let compacted = thread::Entity::find_by_id(second_thread_id)
        .one(db)
        .await?
        .unwrap();
    assert!(compacted.archived);
    assert!(!compacted.active);
    let archived_threads = thread_repository
        .get_archived_threads_by_thread_id("orm-it", Some(vec![9001]))
        .await?;
    assert_eq!(archived_threads.len(), 1);
    let filtered_archived_threads = thread_repository
        .get_archived_threads_by_filter(
            "orm-it",
            Some("archived"),
            (
                Some(Utc::now() - Duration::minutes(1)),
                Some(Utc::now() + Duration::minutes(1)),
            ),
            0,
            10,
        )
        .await?;
    assert_eq!(filtered_archived_threads.len(), 1);

    let notice_repository = NoticeRepositoryImpl::new(db.clone());
    let notice = notice_repository
        .create_notice(
            CreateNoticeInput {
                title: "お知らせ".to_string(),
                slug: "orm-it-notice".to_string(),
                content: "内容".to_string(),
                published_at: now,
                hide_from_list: false,
            },
            Some("admin@example.test".to_string()),
        )
        .await?;
    let updated_notice = notice_repository
        .update_notice(
            notice.id,
            UpdateNoticeInput {
                title: Some("更新お知らせ".to_string()),
                content: Some("更新内容".to_string()),
                published_at: None,
                slug: None,
                hide_from_list: Some(true),
            },
        )
        .await?;
    assert_eq!(updated_notice.title, "更新お知らせ");
    assert!(updated_notice.hide_from_list);
    notice_repository.delete_notice(notice.id).await?;
    assert!(
        notice_repository
            .get_notice_by_id(notice.id)
            .await?
            .is_none()
    );

    let terms_repository = TermsRepositoryImpl::new(db.clone());
    let terms = terms_repository
        .update_terms(
            UpdateTermsInput {
                content: "更新された規約".to_string(),
            },
            Some("admin@example.test".to_string()),
        )
        .await?;
    assert_eq!(terms.content, "更新された規約");
    assert_eq!(terms.updated_by.as_deref(), Some("admin@example.test"));

    let settings_repository = ServerSettingsRepositoryImpl::new(db.clone());
    let setting = settings_repository
        .upsert(crate::models::UpsertServerSettingInput {
            setting_key: "integration.setting".to_string(),
            value: "first".to_string(),
            description: Some("first description".to_string()),
        })
        .await?;
    let upserted_setting = settings_repository
        .upsert(crate::models::UpsertServerSettingInput {
            setting_key: "integration.setting".to_string(),
            value: "second".to_string(),
            description: None,
        })
        .await?;
    assert_eq!(setting.id, upserted_setting.id);
    assert_eq!(setting.created_at, upserted_setting.created_at);
    assert_eq!(upserted_setting.value, "second");
    assert_eq!(upserted_setting.description, None);

    let idp_repository = IdpAdminRepositoryImpl::new(db.clone());
    let idp = idp_repository
        .create(CreateIdpInput {
            idp_name: "integration-idp".to_string(),
            idp_display_name: "Integration IdP".to_string(),
            idp_logo_svg: None,
            oidc_config_url: "https://idp.example.test/.well-known/openid-configuration"
                .to_string(),
            client_id: "client".to_string(),
            client_secret: "secret".to_string(),
            enabled: true,
        })
        .await?;
    let stored_idp = idp::Entity::find_by_id(idp.id).one(db).await?.unwrap();
    assert_ne!(stored_idp.client_secret, "secret");
    let updated_idp = idp_repository
        .update(
            idp.id,
            UpdateIdpInput {
                idp_display_name: Some("Updated IdP".to_string()),
                idp_logo_svg: None,
                oidc_config_url: None,
                client_id: None,
                client_secret: Some("new-secret".to_string()),
                enabled: Some(false),
            },
        )
        .await?;
    assert_eq!(updated_idp.idp_display_name, "Updated IdP");
    assert!(!updated_idp.enabled);

    let captcha_repository = CaptchaConfigRepositoryImpl::new(db.clone());
    let captcha = captcha_repository
        .create(
            CreateCaptchaConfigInput {
                name: "Integration Captcha".to_string(),
                provider: "custom".to_string(),
                site_key: "site-key".to_string(),
                secret: "secret".to_string(),
                base_url: None,
                widget: None,
                capture_fields: vec!["token".to_string()],
                verification: Some(crate::models::CaptchaVerificationConfig {
                    url: Some("https://captcha.example.test/verify".to_string()),
                    method: HttpMethod::Post,
                    request_format: RequestFormat::Json,
                    headers: HashMap::new(),
                    body_template: Some("{token}".to_string()),
                    success_path: "success".to_string(),
                    include_ip: true,
                    negate_success: false,
                    score_threshold: Some(0.5),
                    project_id: None,
                }),
                is_active: true,
                display_order: 1,
                endpoint_usage: "auth_code".to_string(),
            },
            Some("admin@example.test".to_string()),
        )
        .await?;
    let updated_captcha = captcha_repository
        .update(
            captcha.id,
            UpdateCaptchaConfigInput {
                name: None,
                provider: None,
                site_key: None,
                secret: None,
                base_url: Some("https://captcha.example.test".to_string()),
                widget: None,
                capture_fields: Some(vec!["token".to_string(), "ip".to_string()]),
                verification: None,
                is_active: None,
                display_order: None,
                endpoint_usage: None,
            },
            Some("editor@example.test".to_string()),
        )
        .await?;
    assert_eq!(updated_captcha.capture_fields, vec!["token", "ip"]);
    assert!(updated_captcha.verification.is_some());
    assert_eq!(
        updated_captcha.base_url.as_deref(),
        Some("https://captcha.example.test")
    );
    assert!(updated_captcha.widget.is_none());
    let captcha_without_optional_values = captcha_repository
        .create(
            CreateCaptchaConfigInput {
                name: "Nullable Captcha".to_string(),
                provider: "turnstile".to_string(),
                site_key: "site-key-2".to_string(),
                secret: "secret-2".to_string(),
                base_url: None,
                widget: None,
                capture_fields: Vec::new(),
                verification: None,
                is_active: false,
                display_order: 2,
                endpoint_usage: "auth_code".to_string(),
            },
            None,
        )
        .await?;
    assert_eq!(captcha_without_optional_values.base_url, None);
    assert!(captcha_without_optional_values.verification.is_none());
    assert_eq!(
        captcha_without_optional_values.capture_fields,
        Vec::<String>::new()
    );

    let restriction_repository =
        crate::repository::user_restriction_repository::UserRestrictionRepositoryImpl::new(
            db.clone(),
        );
    let restriction = restriction_repository
        .create_rule(CreateUserRestrictionRuleInput {
            name: "integration restriction".to_string(),
            rule_type: RestrictionRuleType::IP,
            rule_value: "192.0.2.1".to_string(),
            expires_at: Some(Utc::now() + Duration::hours(1)),
            created_by_email: "admin@example.test".to_string(),
        })
        .await?;
    restriction_repository
        .update_rule(UpdateUserRestrictionRuleInput {
            id: restriction.id,
            name: Some("updated restriction".to_string()),
            rule_type: Some(RestrictionRuleType::IPCidr),
            rule_value: Some("192.0.2.0/24".to_string()),
            expires_at: Some(None),
        })
        .await?;
    let updated_restriction = restriction_repository
        .get_rule_by_id(restriction.id)
        .await?
        .unwrap();
    assert_eq!(updated_restriction.name, "updated restriction");
    assert_eq!(updated_restriction.rule_type, RestrictionRuleType::IPCidr);
    assert_eq!(updated_restriction.expires_at, None);
    restriction_repository.delete_rule(restriction.id).await?;
    assert!(
        restriction_repository
            .get_rule_by_id(restriction.id)
            .await?
            .is_none()
    );

    let cap_repository = CapRepositoryImpl::new(db.clone());
    let cap = cap_repository
        .create_cap("integration cap", "description", "hash")
        .await?;
    cap_repository
        .update_cap(
            cap.id,
            Some("should rollback"),
            None,
            None,
            Some(vec![Uuid::now_v7()]),
        )
        .await
        .expect_err("invalid board relation must rollback the cap update");
    let rolled_back_cap = cap_repository
        .get_caps()
        .await?
        .into_iter()
        .find(|value| value.id == cap.id)
        .unwrap();
    assert_eq!(rolled_back_cap.name, "integration cap");
    assert!(rolled_back_cap.board_ids.is_empty());
    let updated_cap = cap_repository
        .update_cap(
            cap.id,
            Some("updated cap"),
            None,
            None,
            Some(vec![board.id, second_board.id]),
        )
        .await?;
    assert_eq!(updated_cap.board_ids.len(), 2);
    cap_repository.delete_cap(cap.id).await?;
    assert!(
        cap_repository
            .get_caps()
            .await?
            .into_iter()
            .all(|value| value.id != cap.id)
    );
    assert_eq!(
        board_cap::Entity::find()
            .filter(board_cap::Column::CapId.eq(cap.id))
            .all(db)
            .await?
            .len(),
        0
    );

    let ng_word_repository = NgWordRepositoryImpl::new(db.clone());
    let ng_word = ng_word_repository
        .create_ng_word("integration ng", "bad-word")
        .await?;
    let updated_ng_word = ng_word_repository
        .update_ng_word(
            ng_word.id,
            Some("updated ng"),
            None,
            Some(vec![second_board.id]),
        )
        .await?;
    assert_eq!(updated_ng_word.board_ids, vec![second_board.id]);
    ng_word_repository.delete_ng_word(ng_word.id).await?;
    assert!(
        ng_word_repository
            .get_ng_words()
            .await?
            .into_iter()
            .all(|value| value.id != ng_word.id)
    );
    assert_eq!(
        board_ng_word::Entity::find()
            .filter(board_ng_word::Column::NgWordId.eq(ng_word.id))
            .all(db)
            .await?
            .len(),
        0
    );

    let untouched_board = board_repository
        .edit_board("orm-it", empty_edit_board_input())
        .await?;
    assert_eq!(untouched_board.name, edited_board.name);
    assert_eq!(untouched_board.default_name, edited_board.default_name);
    let untouched_info = board_repository.get_board_info(board.id).await?;
    assert_eq!(
        untouched_info.base_thread_creation_span_sec,
        edited_info.base_thread_creation_span_sec
    );
    assert_eq!(untouched_info.custom_1001_message, None);

    assert_not_found(
        board_repository
            .edit_board("no-such-board", empty_edit_board_input())
            .await
            .expect_err("edit_board on a missing board must fail"),
        "Board not found",
    );
    assert_not_found(
        response_repository
            .update_res(
                Uuid::now_v7(),
                Some("編集者".to_string()),
                None,
                None,
                None,
                None,
            )
            .await
            .expect_err("update_res on a missing response must fail"),
        "Response not found",
    );
    assert_not_found(
        cap_repository
            .update_cap(cap.id, Some("gone"), None, None, None)
            .await
            .expect_err("update_cap on a deleted cap must fail"),
        "Cap not found",
    );
    assert_not_found(
        ng_word_repository
            .update_ng_word(ng_word.id, Some("gone"), None, None)
            .await
            .expect_err("update_ng_word on a deleted ng word must fail"),
        "NG word not found",
    );

    let user_id = Uuid::now_v7();
    user::ActiveModel {
        id: Set(user_id),
        user_name: Set("integration user".to_string()),
        enabled: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await?;
    user_idp_binding::ActiveModel {
        id: Set(Uuid::now_v7()),
        user_id: Set(user_id),
        idp_id: Set(idp.id),
        idp_sub: Set("subject".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await?;
    user_authed_token::ActiveModel {
        id: Set(Uuid::now_v7()),
        user_id: Set(user_id),
        authed_token_id: Set(token_id),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await?;
    let user_repository = AdminUserRepositoryImpl::new(db.clone());
    let users = user_repository
        .search_users(None, None, Some(token_id))
        .await?;
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].id, user_id);
    assert_eq!(users[0].idp_bindings.len(), 1);
    assert_eq!(users[0].authed_token_ids, vec![token_id]);
    let token_repository = AuthedTokenRepositoryImpl::new(db.clone());
    token_repository.delete_authed_token(token_id).await?;
    assert!(!token_repository.get_authed_token(token_id).await?.validity);
    user_repository.update_user_status(user_id, false).await?;
    assert!(
        !user::Entity::find_by_id(user_id)
            .one(db)
            .await?
            .unwrap()
            .enabled
    );
    assert!(
        !authed_token::Entity::find_by_id(token_id)
            .one(db)
            .await?
            .unwrap()
            .validity
    );

    token_repository.set_require_reauth(token_id).await?;
    assert!(
        token_repository
            .get_authed_token(token_id)
            .await?
            .require_reauth
    );
    token_repository.clear_require_reauth(token_id).await?;
    assert!(
        !token_repository
            .get_authed_token(token_id)
            .await?
            .require_reauth
    );
    let (tokens, total) = token_repository
        .list_authed_tokens(
            crate::repository::authed_token_repository::ListAuthedTokensParams {
                offset: 0,
                limit: 10,
                origin_ip: Some("127.0.0.1"),
                writing_ua: None,
                authed_ua: None,
                asn_num: Some(64512),
                validity: Some(false),
                sort_column: "created_at",
                sort_asc: true,
            },
        )
        .await?;
    assert_eq!(total, 1);
    assert_eq!(tokens.len(), 1);

    user_idp_binding::Entity::delete_many()
        .filter(user_idp_binding::Column::UserId.eq(user_id))
        .exec(db)
        .await?;
    user_authed_token::Entity::delete_many()
        .filter(user_authed_token::Column::UserId.eq(user_id))
        .exec(db)
        .await?;
    user::Entity::delete_by_id(user_id).exec(db).await?;
    idp_repository.delete(idp.id).await?;
    assert!(idp_repository.get_by_id(idp.id).await?.is_none());
    captcha_repository.delete(captcha.id).await?;
    assert!(captcha_repository.get_by_id(captcha.id).await?.is_none());

    Ok(())
}

/// Pins the relational read paths to the semantics of the pre-SeaORM queries:
/// `find_with_related` must behave exactly like the original LEFT OUTER JOIN,
/// including rows with no relations at all and duplicate junction rows (the
/// junction tables carry no UNIQUE constraint).
#[tokio::test]
async fn relational_reads_match_left_outer_join() -> anyhow::Result<()> {
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

    let test_db = setup_database().await?;
    let db = &test_db.db;

    let board_repository = AdminBoardRepositoryImpl::new(db.clone());
    let b1 = board_repository
        .create_board(create_board_input("rel-1", "B1"))
        .await?;
    let b2 = board_repository
        .create_board(create_board_input("rel-2", "B2"))
        .await?;

    async fn left_join_map(
        db: &DatabaseConnection,
        sql: &str,
    ) -> anyhow::Result<HashMap<Uuid, Vec<Uuid>>> {
        let mut map = HashMap::<Uuid, Vec<Uuid>>::new();
        for row in db
            .query_all_raw(Statement::from_string(DatabaseBackend::MySql, sql))
            .await?
        {
            let id: Uuid = row.try_get("", "id")?;
            let board_id: Option<Uuid> = row.try_get("", "board_id")?;
            let entry = map.entry(id).or_default();
            if let Some(board_id) = board_id {
                entry.push(board_id);
            }
        }
        for ids in map.values_mut() {
            ids.sort();
        }
        Ok(map)
    }

    // NG words: multi-board, zero-board, duplicated-board
    let ng_repository = NgWordRepositoryImpl::new(db.clone());
    let ng_multi = ng_repository.create_ng_word("A-multi", "wa").await?;
    ng_repository.create_ng_word("B-zero", "wb").await?;
    let ng_dup = ng_repository.create_ng_word("C-dup", "wc").await?;
    ng_repository
        .update_ng_word(ng_multi.id, None, None, Some(vec![b1.id, b2.id]))
        .await?;
    ng_repository
        .update_ng_word(ng_dup.id, None, None, Some(vec![b1.id, b1.id]))
        .await?;

    let ng_words = ng_repository.get_ng_words().await?;
    let mut ng_new = HashMap::<Uuid, Vec<Uuid>>::new();
    for row in &ng_words {
        let mut ids = row.board_ids.clone();
        ids.sort();
        ng_new.insert(row.id, ids);
    }
    let ng_old = left_join_map(
        db,
        r#"SELECT ng.id AS id, bng.board_id AS board_id
           FROM ng_words AS ng
           LEFT OUTER JOIN boards_ng_words AS bng ON ng.id = bng.ng_word_id"#,
    )
    .await?;
    assert_eq!(ng_new, ng_old, "get_ng_words diverged from LEFT OUTER JOIN");
    assert_eq!(ng_new.get(&ng_multi.id).map(Vec::len), Some(2));
    assert_eq!(
        ng_new.get(&ng_dup.id).map(Vec::len),
        Some(2),
        "duplicate junction rows must be preserved"
    );
    assert!(ng_words.iter().any(|w| w.board_ids.is_empty()));
    let names = ng_words.iter().map(|w| w.name.clone()).collect::<Vec<_>>();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "find_with_related must keep the ORDER BY");

    // Caps: same three shapes
    let cap_repository = CapRepositoryImpl::new(db.clone());
    let cap_multi = cap_repository.create_cap("A-multi", "d", "h").await?;
    cap_repository.create_cap("B-zero", "d", "h").await?;
    let cap_dup = cap_repository.create_cap("C-dup", "d", "h").await?;
    cap_repository
        .update_cap(cap_multi.id, None, None, None, Some(vec![b1.id, b2.id]))
        .await?;
    cap_repository
        .update_cap(cap_dup.id, None, None, None, Some(vec![b1.id, b1.id]))
        .await?;

    let caps = cap_repository.get_caps().await?;
    let mut cap_new = HashMap::<Uuid, Vec<Uuid>>::new();
    for row in &caps {
        let mut ids = row.board_ids.clone();
        ids.sort();
        cap_new.insert(row.id, ids);
    }
    let cap_old = left_join_map(
        db,
        r#"SELECT c.id AS id, bc.board_id AS board_id
           FROM caps AS c
           LEFT OUTER JOIN boards_caps AS bc ON c.id = bc.cap_id"#,
    )
    .await?;
    assert_eq!(cap_new, cap_old, "get_caps diverged from LEFT OUTER JOIN");
    assert_eq!(cap_new.get(&cap_dup.id).map(Vec::len), Some(2));
    assert!(caps.iter().any(|c| c.board_ids.is_empty()));

    Ok(())
}
