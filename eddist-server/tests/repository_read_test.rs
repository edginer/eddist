//! Behavior checks shared by the SQLx baseline and the incremental SeaORM migration.
use chrono::{NaiveDate, NaiveDateTime};
use eddist::repositories::{
    bbs_repository::{BbsRepositoryImpl, BoardRepository},
    captcha_config_repository::{CaptchaConfigRepository, CaptchaConfigRepositoryImpl},
    idp_repository::{IdpRepository, IdpRepositoryImpl},
    notice_repository::{NoticeRepository, NoticeRepositoryImpl},
    server_settings_repository::{ServerSettingsRepository, ServerSettingsRepositoryImpl},
    stats_repository::{StatsRepository, StatsRepositoryImpl},
    terms_repository::{TermsRepository, TermsRepositoryImpl},
    user_restriction_repository::{UserRestrictionRepository, UserRestrictionRepositoryImpl},
};
use eddist_core::{
    domain::user_restriction::{RestrictionRuleType, RestrictionTarget},
    server_settings::ServerSettingKey,
};
use sqlx::{MySqlPool, mysql::MySqlPoolOptions};
use std::time::Duration;
use testcontainers::{ContainerAsync, ImageExt, core::IntoContainerPort, runners::AsyncRunner};
use testcontainers_modules::mysql::Mysql;
use uuid::Uuid;

struct TestDatabase {
    pool: MySqlPool,
    _container: ContainerAsync<Mysql>,
}

impl TestDatabase {
    fn bbs(&self) -> BbsRepositoryImpl {
        BbsRepositoryImpl::new(self.pool.clone(), self.orm())
    }

    async fn insert_board(&self, key: &str) -> Uuid {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO boards (id, name, board_key, default_name) VALUES (UUID_TO_BIN(?), ?, ?, ?)")
            .bind(id.to_string()).bind(format!("板 {key}")).bind(key).bind("名無さん")
            .execute(&self.pool).await.unwrap();
        id
    }

    fn orm(&self) -> sea_orm::DatabaseConnection {
        sea_orm::SqlxMySqlConnector::from_sqlx_mysql_pool(self.pool.clone())
    }

    async fn new() -> Self {
        Self::with_max_connections(1).await
    }

    async fn with_max_connections(max_connections: u32) -> Self {
        let container = Mysql::default().with_tag("8.0").start().await.unwrap();
        let port = container.get_host_port_ipv4(3306.tcp()).await.unwrap();
        let pool = MySqlPoolOptions::new()
            .max_connections(max_connections)
            .acquire_timeout(Duration::from_secs(5))
            .after_connect(|conn, _| {
                Box::pin(async move {
                    use sqlx::Executor;
                    conn.execute("SET SESSION TRANSACTION ISOLATION LEVEL READ COMMITTED")
                        .await?;
                    conn.execute(
                        "SET SESSION sql_mode = CONCAT(@@sql_mode, ',TIME_TRUNCATE_FRACTIONAL')",
                    )
                    .await?;
                    Ok(())
                })
            })
            .connect(&format!("mysql://root@127.0.0.1:{port}/test"))
            .await
            .unwrap();
        sqlx::migrate!("../migrations").run(&pool).await.unwrap();
        Self {
            pool,
            _container: container,
        }
    }
}

fn time(day: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2020, 1, day)
        .unwrap()
        .and_hms_milli_opt(12, 34, 56, 123)
        .unwrap()
}

#[tokio::test]
async fn stats_preserve_jst_calendar_date_range_and_order() {
    let db = TestDatabase::new().await;
    let repo = StatsRepositoryImpl::new(db.orm());
    // Record the SQLx calendar contract independently of the host's timezone.
    for (utc, expected) in [
        ("2020-01-01T14:59:59Z", "2020-01-01"),
        ("2020-01-01T15:00:00Z", "2020-01-02"),
        ("2020-12-31T15:00:00Z", "2021-01-01"),
        ("2020-02-28T15:00:00Z", "2020-02-29"),
    ] {
        let timestamp = chrono::DateTime::parse_from_rfc3339(utc)
            .unwrap()
            .timestamp();
        sqlx::query("SET timestamp = ?")
            .bind(timestamp)
            .execute(&db.pool)
            .await
            .unwrap();
        let date: NaiveDate =
            sqlx::query_scalar("SELECT DATE(CONVERT_TZ(NOW(), '+00:00', '+09:00'))")
                .fetch_one(&db.pool)
                .await
                .unwrap();
        assert_eq!(date.to_string(), expected);
    }
    sqlx::query("SET timestamp = 0")
        .execute(&db.pool)
        .await
        .unwrap();
    let today: NaiveDate = sqlx::query_scalar("SELECT DATE(CONVERT_TZ(NOW(), '+00:00', '+09:00'))")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert!(repo.get_today_stats_per_board().await.unwrap().is_empty());
    assert!(repo.get_daily_stats_per_board(30).await.unwrap().is_empty());
    for (key, days_ago, responses, threads) in [
        ("a", 3, 30_i64, 3_i64),
        ("b", 2, 21, 3),
        ("a", 2, 20, 2),
        ("a", 1, 10, 1),
        ("a", 0, 100, 10),
        ("future", -1, 200, 20),
    ] {
        sqlx::query("INSERT INTO daily_stats (date, board_key, total_responses, new_threads) VALUES (?, ?, ?, ?)")
            .bind(today - chrono::Duration::days(days_ago)).bind(key).bind(responses).bind(threads)
            .execute(&db.pool).await.unwrap();
    }
    let rows = repo.get_today_stats_per_board().await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].date, today);
    assert_eq!(rows[0].board_key, "a");
    assert_eq!(rows[0].total_responses, 100);
    assert_eq!(rows[0].new_threads, 10);
    let rows = repo.get_daily_stats_per_board(2).await.unwrap();
    let values: Vec<_> = rows
        .into_iter()
        .map(|r| (r.board_key, r.date, r.total_responses, r.new_threads))
        .collect();
    assert_eq!(
        values,
        [
            ("a".into(), today - chrono::Duration::days(1), 10, 1),
            ("a".into(), today - chrono::Duration::days(2), 20, 2),
            ("b".into(), today - chrono::Duration::days(2), 21, 3),
        ]
    );
    assert!(repo.get_daily_stats_per_board(0).await.unwrap().is_empty());
    assert!(
        repo.get_daily_stats_per_board(u32::MAX)
            .await
            .unwrap()
            .is_empty()
    );
    db.pool.close().await;
    assert!(repo.get_today_stats_per_board().await.is_err());
    assert!(repo.get_daily_stats_per_board(2).await.is_err());
    assert!(repo.flush_board_stats(&[("a".into(), 1, 1)]).await.is_err());
}

#[tokio::test]
async fn stats_flush_adds_per_board_and_rolls_back_the_entire_snapshot() {
    let db = TestDatabase::with_max_connections(4).await;
    let mut orm = db.orm();
    let statements = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    orm.set_metric_callback({
        let statements = statements.clone();
        move |info| statements.lock().unwrap().push(info.statement.sql.clone())
    });
    let repo = StatsRepositoryImpl::new(orm);
    repo.flush_board_stats(&[("a".into(), 10, 2), ("b".into(), 20, 3)])
        .await
        .unwrap();
    // No model reload SELECT is needed: one DML statement per snapshot entry.
    {
        let sql = statements.lock().unwrap();
        assert_eq!(sql.len(), 2);
        assert!(sql.iter().all(|sql| sql.starts_with("INSERT INTO")));
    }
    sqlx::query("UPDATE daily_stats SET created_at = ? WHERE board_key = 'a'")
        .bind(time(1))
        .execute(&db.pool)
        .await
        .unwrap();
    repo.flush_board_stats(&[("a".into(), 3, 1), ("a".into(), -1, 0), ("c".into(), 5, 0)])
        .await
        .unwrap();
    repo.flush_board_stats(&[]).await.unwrap();
    repo.flush_board_stats(&[("a".into(), 0, 0)]).await.unwrap();
    let left = [("a".into(), 2, 1)];
    let right = [("a".into(), 3, 1)];
    let (left, right) = tokio::join!(
        repo.flush_board_stats(&left),
        repo.flush_board_stats(&right)
    );
    left.unwrap();
    right.unwrap();
    let mut before = repo.get_today_stats_per_board().await.unwrap();
    before.sort_by(|a, b| a.board_key.cmp(&b.board_key));
    let before: Vec<_> = before
        .into_iter()
        .map(|r| (r.board_key, r.total_responses, r.new_threads))
        .collect();
    assert_eq!(
        before,
        [("a".into(), 17, 5), ("b".into(), 20, 3), ("c".into(), 5, 0)]
    );

    // The second insert fails after the first update; neither may be committed.
    assert!(
        repo.flush_board_stats(&[("a".into(), 100, 10), ("x".repeat(256), 1, 1)])
            .await
            .is_err()
    );
    let mut after = repo.get_today_stats_per_board().await.unwrap();
    after.sort_by(|a, b| a.board_key.cmp(&b.board_key));
    let after: Vec<_> = after
        .into_iter()
        .map(|r| (r.board_key, r.total_responses, r.new_threads))
        .collect();
    assert_eq!(after, before);
    let created_at: NaiveDateTime =
        sqlx::query_scalar("SELECT created_at FROM daily_stats WHERE board_key = 'a'")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(created_at, time(1));
}

#[tokio::test]
async fn boards_preserve_all_settings_nullable_columns_and_missing_results() {
    let db = TestDatabase::new().await;
    let repo = db.bbs();
    let id = db.insert_board("boardtest").await;
    let no_info = db.insert_board("noinfo").await;
    let board = repo.get_board("boardtest").await.unwrap().unwrap();
    assert_eq!(board.id, id);
    assert_eq!(board.name, "板 boardtest");
    assert_eq!(board.board_key, "boardtest");
    assert_eq!(board.default_name, "名無さん");
    let boards = repo.get_boards().await.unwrap();
    assert_eq!(boards.len(), 3); // Includes the migration's experiment board.
    let listed = boards.iter().find(|b| b.id == id).unwrap();
    assert_eq!(listed.name, board.name);
    assert_eq!(listed.board_key, board.board_key);
    assert_eq!(listed.default_name, board.default_name);
    assert!(repo.get_board("missing").await.unwrap().is_none());
    assert!(repo.get_board_info(no_info).await.unwrap().is_none());
    assert!(repo.get_board_info(Uuid::nil()).await.unwrap().is_none());

    sqlx::query("INSERT INTO boards_info (id, local_rules, base_thread_creation_span_sec, base_response_creation_span_sec, max_thread_name_byte_length, max_author_name_byte_length, max_email_byte_length, max_response_body_byte_length, max_response_body_lines, threads_archive_cron, threads_archive_trigger_thread_count, created_at, updated_at, read_only, force_metadent_type, enable_1001_message, custom_1001_message) VALUES (UUID_TO_BIN(?), ?, 121, 6, 257, 129, 130, 9193, 33, ?, 501, ?, ?, TRUE, 'vvv', FALSE, ?)")
        .bind(id.to_string()).bind("板ルール\n日本語").bind("0 0 * * * *")
        .bind(time(1)).bind(time(2)).bind("おしまいです")
        .execute(&db.pool).await.unwrap();
    let info = repo.get_board_info(id).await.unwrap().unwrap();
    assert_eq!(info.id, id);
    assert_eq!(info.local_rules, "板ルール\n日本語");
    assert_eq!(info.base_thread_creation_span_sec, 121);
    assert_eq!(info.base_response_creation_span_sec, 6);
    assert_eq!(info.max_thread_name_byte_length, 257);
    assert_eq!(info.max_author_name_byte_length, 129);
    assert_eq!(info.max_email_byte_length, 130);
    assert_eq!(info.max_response_body_byte_length, 9193);
    assert_eq!(info.max_response_body_lines, 33);
    assert_eq!(info.threads_archive_cron.as_deref(), Some("0 0 * * * *"));
    assert_eq!(info.threads_archive_trigger_thread_count, Some(501));
    assert_eq!(info.created_at, time(1));
    assert_eq!(info.updated_at, time(2));
    assert!(info.read_only);
    assert_eq!(info.force_metadent_type.as_deref(), Some("vvv"));
    assert!(!info.enable_1001_message);
    assert_eq!(info.custom_1001_message.as_deref(), Some("おしまいです"));

    sqlx::query("INSERT INTO boards_info (id, local_rules, created_at, updated_at) VALUES (UUID_TO_BIN(?), '', ?, ?)")
        .bind(no_info.to_string()).bind(time(1)).bind(time(1)).execute(&db.pool).await.unwrap();
    let defaults = repo.get_board_info(no_info).await.unwrap().unwrap();
    assert!(!defaults.read_only);
    assert!(defaults.enable_1001_message);
    assert!(defaults.threads_archive_cron.is_none());
    assert!(defaults.threads_archive_trigger_thread_count.is_none());
    assert!(defaults.force_metadent_type.is_none());
    assert!(defaults.custom_1001_message.is_none());

    db.pool.close().await;
    assert!(repo.get_boards().await.is_err());
    assert!(repo.get_board("missing").await.is_err());
    assert!(repo.get_board_info(id).await.is_err());
    assert!(repo.get_ng_words_by_board_key("missing").await.is_err());
    assert!(repo.get_cap_by_board_key("hash", "missing").await.is_err());
}

#[tokio::test]
async fn board_joins_preserve_board_isolation_and_duplicate_bindings() {
    let db = TestDatabase::new().await;
    let repo = db.bbs();
    let first = db.insert_board("first").await;
    let second = db.insert_board("second").await;
    let mut words = Vec::new();
    for (name, word) in [
        ("共通", "禁止語"),
        ("別板", "別の禁止語"),
        ("未設定", "未使用語"),
    ] {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO ng_words (id, name, word, created_at, updated_at) VALUES (UUID_TO_BIN(?), ?, ?, ?, ?)")
            .bind(id.to_string()).bind(name).bind(word).bind(time(1)).bind(time(2))
            .execute(&db.pool).await.unwrap();
        words.push(id);
    }
    // Keep duplicate association rows: the SQLx JOIN returns one result per binding.
    for (board, word) in [
        (first, words[0]),
        (first, words[0]),
        (second, words[0]),
        (second, words[1]),
    ] {
        sqlx::query("INSERT INTO boards_ng_words (id, board_id, ng_word_id) VALUES (UUID_TO_BIN(?), UUID_TO_BIN(?), UUID_TO_BIN(?))")
            .bind(Uuid::now_v7().to_string()).bind(board.to_string()).bind(word.to_string())
            .execute(&db.pool).await.unwrap();
    }
    let found = repo.get_ng_words_by_board_key("first").await.unwrap();
    assert_eq!(found.len(), 2);
    for word in found {
        assert_eq!(word.id, words[0]);
        assert_eq!(word.name, "共通");
        assert_eq!(word.word, "禁止語");
        assert_eq!(word.created_at, time(1));
        assert_eq!(word.updated_at, time(2));
    }
    let mut found = repo.get_ng_words_by_board_key("second").await.unwrap();
    found.sort_by_key(|w| w.id);
    let mut expected = words[..2].to_vec();
    expected.sort();
    assert_eq!(found.iter().map(|w| w.id).collect::<Vec<_>>(), expected);
    assert!(
        repo.get_ng_words_by_board_key("experiment")
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        repo.get_ng_words_by_board_key("missing")
            .await
            .unwrap()
            .is_empty()
    );

    let mut caps = Vec::new();
    for (name, hash) in [
        ("共通キャップ", "shared-hash"),
        ("別板キャップ", "second-hash"),
        ("未設定", "unbound-hash"),
    ] {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO caps (id, name, description, password_hash, created_at, updated_at) VALUES (UUID_TO_BIN(?), ?, ?, ?, ?, ?)")
            .bind(id.to_string()).bind(name).bind("キャップの説明").bind(hash).bind(time(1)).bind(time(2))
            .execute(&db.pool).await.unwrap();
        caps.push(id);
    }
    for (board, cap) in [
        (first, caps[0]),
        (first, caps[0]),
        (second, caps[0]),
        (second, caps[1]),
    ] {
        sqlx::query("INSERT INTO boards_caps (id, board_id, cap_id) VALUES (UUID_TO_BIN(?), UUID_TO_BIN(?), UUID_TO_BIN(?))")
            .bind(Uuid::now_v7().to_string()).bind(board.to_string()).bind(cap.to_string())
            .execute(&db.pool).await.unwrap();
    }
    let cap = repo
        .get_cap_by_board_key("shared-hash", "first")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(cap.id, caps[0]);
    assert_eq!(cap.name, "共通キャップ");
    assert_eq!(cap.description, "キャップの説明");
    assert_eq!(cap.password_hash, "shared-hash");
    assert_eq!(cap.created_at, time(1));
    assert_eq!(cap.updated_at, time(2));
    assert_eq!(
        repo.get_cap_by_board_key("shared-hash", "second")
            .await
            .unwrap()
            .unwrap()
            .id,
        caps[0]
    );
    assert_eq!(
        repo.get_cap_by_board_key("second-hash", "second")
            .await
            .unwrap()
            .unwrap()
            .id,
        caps[1]
    );
    for (hash, key) in [
        ("second-hash", "first"),
        ("unbound-hash", "first"),
        ("shared-hash", "experiment"),
        ("shared-hash", "missing"),
        ("wrong-hash", "second"),
    ] {
        assert!(
            repo.get_cap_by_board_key(hash, key)
                .await
                .unwrap()
                .is_none()
        );
    }
}

#[tokio::test]
async fn notices_preserve_visibility_pagination_and_optional_fields() {
    let db = TestDatabase::new().await;
    let repo = NoticeRepositoryImpl::new(db.orm());
    assert!(repo.get_notices_paginated(0, 2).await.unwrap().is_empty());
    assert_eq!(repo.count_notices().await.unwrap(), 0);
    assert!(repo.get_notice_by_slug("missing").await.unwrap().is_none());

    let mut ids = Vec::new();
    for (slug, published_at, hidden, email) in [
        ("old", time(1), false, None),
        ("new", time(3), false, Some("author@example.com")),
        ("middle", time(2), false, None),
        ("hidden", time(4), true, None),
        (
            "future",
            NaiveDate::from_ymd_opt(2099, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            false,
            None,
        ),
    ] {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO notices (id, slug, title, content, created_at, updated_at, published_at, author_email, hide_from_list) VALUES (UUID_TO_BIN(?), ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(id.to_string()).bind(slug).bind(format!("お知らせ {slug}"))
            .bind("本文\n日本語").bind(time(1)).bind(time(2)).bind(published_at)
            .bind(email).bind(hidden).execute(&db.pool).await.unwrap();
        ids.push(id);
    }

    assert_eq!(repo.count_notices().await.unwrap(), 3);
    let page = repo.get_notices_paginated(0, 2).await.unwrap();
    assert_eq!(
        page.iter().map(|n| n.slug.as_str()).collect::<Vec<_>>(),
        ["new", "middle"]
    );
    assert_eq!(page[0].id, ids[1]);
    assert_eq!(page[0].title, "お知らせ new");
    assert_eq!(page[0].published_at, time(3));
    let page = repo.get_notices_paginated(1, 2).await.unwrap();
    assert_eq!(page.len(), 1);
    assert_eq!(page[0].slug, "old");
    assert!(repo.get_notices_paginated(2, 2).await.unwrap().is_empty());
    assert!(repo.get_notices_paginated(0, 0).await.unwrap().is_empty());
    assert!(repo.get_notice_by_slug("future").await.unwrap().is_none());

    let hidden = repo.get_notice_by_slug("hidden").await.unwrap().unwrap();
    assert_eq!(hidden.id, ids[3]);
    assert!(hidden.hide_from_list);
    assert_eq!(hidden.author_email, None);
    assert_eq!(hidden.content, "本文\n日本語");
    assert_eq!(hidden.created_at, time(1));
    assert_eq!(hidden.updated_at, time(2));
    assert_eq!(hidden.published_at, time(4));
    let visible = repo.get_notice_by_slug("new").await.unwrap().unwrap();
    assert!(!visible.hide_from_list);
    assert_eq!(visible.author_email.as_deref(), Some("author@example.com"));

    // Preserve NOW()'s DB clock and second precision at the publication boundary.
    sqlx::query("SET timestamp = 1893456000")
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE notices SET published_at = NOW() WHERE slug = 'future'")
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(repo.get_notice_by_slug("future").await.unwrap().is_some());
    sqlx::query("UPDATE notices SET published_at = DATE_ADD(NOW(), INTERVAL 1000 MICROSECOND) WHERE slug = 'future'").execute(&db.pool).await.unwrap();
    assert!(repo.get_notice_by_slug("future").await.unwrap().is_none());
}

#[tokio::test]
async fn terms_select_latest_update_and_preserve_empty_result() {
    let db = TestDatabase::new().await;
    sqlx::query("DELETE FROM terms")
        .execute(&db.pool)
        .await
        .unwrap();
    let repo = TermsRepositoryImpl::new(db.orm());
    assert!(repo.get_terms().await.unwrap().is_none());

    let mut latest_id = Uuid::nil();
    for (day, updated_by) in [(3, Some("editor@example.com")), (1, None), (2, None)] {
        let id = Uuid::now_v7();
        if day == 3 {
            latest_id = id;
        }
        sqlx::query("INSERT INTO terms (id, content, created_at, updated_at, updated_by) VALUES (UUID_TO_BIN(?), ?, ?, ?, ?)")
            .bind(id.to_string()).bind(format!("規約 {day}")).bind(time(1)).bind(time(day)).bind(updated_by)
            .execute(&db.pool).await.unwrap();
    }
    let terms = repo.get_terms().await.unwrap().unwrap();
    assert_eq!(terms.id, latest_id);
    assert_eq!(terms.content, "規約 3");
    assert_eq!(terms.created_at, time(1));
    assert_eq!(terms.updated_at, time(3));
    assert_eq!(terms.updated_by.as_deref(), Some("editor@example.com"));
    sqlx::query("UPDATE terms SET updated_by = NULL WHERE id = UUID_TO_BIN(?)")
        .bind(latest_id.to_string())
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(
        repo.get_terms()
            .await
            .unwrap()
            .unwrap()
            .updated_by
            .is_none()
    );
    db.pool.close().await;
    assert!(repo.get_terms().await.is_err());
}

#[tokio::test]
async fn settings_and_idps_preserve_values_and_disabled_providers() {
    let db = TestDatabase::new().await;
    let settings = ServerSettingsRepositoryImpl::new(db.orm());
    assert!(
        settings
            .get_all()
            .await
            .unwrap()
            .get(ServerSettingKey::CloseNewAuthentication)
            .is_none()
    );
    sqlx::query("INSERT INTO server_settings (id, setting_key, value, created_at, updated_at) VALUES (UUID_TO_BIN(?), 'bbs.close_new_authentication', 'true', ?, ?)")
        .bind(Uuid::now_v7().to_string()).bind(time(1)).bind(time(1)).execute(&db.pool).await.unwrap();
    assert!(
        settings
            .get_all()
            .await
            .unwrap()
            .is_enabled(ServerSettingKey::CloseNewAuthentication)
    );
    sqlx::query("UPDATE server_settings SET value = 'false'")
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(
        !settings
            .get_all()
            .await
            .unwrap()
            .is_enabled(ServerSettingKey::CloseNewAuthentication)
    );

    let idps = IdpRepositoryImpl::new(db.orm());
    assert!(idps.get_idps().await.unwrap().is_empty());
    let mut ids = Vec::new();
    for (name, enabled, logo) in [("enabled", true, Some("<svg/>")), ("disabled", false, None)] {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO idps (id, idp_name, idp_display_name, idp_logo_svg, oidc_config_url, client_id, client_secret, enabled) VALUES (UUID_TO_BIN(?), ?, ?, ?, ?, ?, ?, ?)")
            .bind(id.to_string()).bind(name).bind(format!("認証 {name}")).bind(logo)
            .bind("https://idp.example/.well-known/openid-configuration").bind("client-id")
            .bind("encrypted-secret").bind(enabled).execute(&db.pool).await.unwrap();
        ids.push(id);
    }
    let mut rows = idps.get_idps().await.unwrap();
    rows.sort_by_key(|idp| !idp.enabled);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].id, ids[0]);
    assert_eq!(rows[1].id, ids[1]);
    assert_eq!(rows[0].idp_name, "enabled");
    assert_eq!(rows[0].idp_display_name, "認証 enabled");
    assert_eq!(rows[0].idp_logo_svg.as_deref(), Some("<svg/>"));
    assert!(rows[1].idp_logo_svg.is_none());
    assert_eq!(
        rows[0].oidc_config_url,
        "https://idp.example/.well-known/openid-configuration"
    );
    assert_eq!(rows[0].client_id, "client-id");
    assert_eq!(rows[0].client_secret, "encrypted-secret");
    assert!(!rows[1].enabled);
    db.pool.close().await;
    assert!(settings.get_all().await.is_err());
    assert!(idps.get_idps().await.is_err());
}

#[tokio::test]
async fn restrictions_preserve_expiry_boundary_order_and_enum_mapping() {
    let db = TestDatabase::new().await;
    let repo = UserRestrictionRepositoryImpl::new(db.orm());
    assert!(repo.get_all_active_rules().await.unwrap().is_empty());
    sqlx::query("SET timestamp = 1893456000")
        .execute(&db.pool)
        .await
        .unwrap();
    let mut ids = Vec::new();
    for (day, expires_at, rule_type, target) in [
        (1, None, "IP", "BOTH"),
        (3, Some("2030-01-01 00:00:00.001"), "ASN", "POSTING"),
        (
            2,
            Some("2029-12-31 23:59:59.999"),
            "IP_CIDR",
            "AUTHENTICATION",
        ),
        (4, Some("2030-01-01 00:00:00.000"), "USER_AGENT", "BOTH"),
    ] {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO user_restriction_rules (id, name, rule_type, target, rule_value, expires_at, created_at, updated_at, created_by_email) VALUES (UUID_TO_BIN(?), ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(id.to_string()).bind(format!("規制 {day}")).bind(rule_type).bind(target).bind("123")
            .bind(expires_at).bind(time(day)).bind(time(4)).bind("admin@example.com").execute(&db.pool).await.unwrap();
        ids.push(id);
    }
    let rules = repo.get_all_active_rules().await.unwrap();
    assert_eq!(
        rules.iter().map(|r| r.id).collect::<Vec<_>>(),
        [ids[1], ids[0]]
    );
    assert_eq!(rules[0].name, "規制 3");
    assert_eq!(rules[0].rule_type, RestrictionRuleType::Asn);
    assert_eq!(rules[0].target, RestrictionTarget::Posting);
    assert_eq!(rules[0].rule_value, "123");
    assert_eq!(rules[0].created_at, time(3).and_utc());
    assert_eq!(rules[0].updated_at, time(4).and_utc());
    assert_eq!(rules[0].created_by_email, "admin@example.com");
    assert_eq!(rules[0].expires_at.unwrap().timestamp_subsec_millis(), 1);
    assert_eq!(rules[1].rule_type, RestrictionRuleType::IP);
    assert_eq!(rules[1].target, RestrictionTarget::Both);
    assert!(rules[1].expires_at.is_none());
    sqlx::query("UPDATE user_restriction_rules SET target = 'invalid' WHERE id = UUID_TO_BIN(?)")
        .bind(ids[0].to_string())
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(repo.get_all_active_rules().await.is_err());
    db.pool.close().await;
    assert!(repo.get_all_active_rules().await.is_err());
}

#[tokio::test]
async fn captcha_preserves_active_filter_order_json_and_invalid_snapshot_handling() {
    let db = TestDatabase::new().await;
    let repo = CaptchaConfigRepositoryImpl::new(db.orm());
    assert!(repo.get_active_captcha_configs().await.unwrap().is_empty());
    for (name, order, day, active) in [
        ("second", 1, 2, true),
        ("first", 1, 1, true),
        ("last", 2, 1, true),
        ("inactive", 0, 1, false),
    ] {
        sqlx::query("INSERT INTO captcha_configs (id, name, provider, site_key, secret, capture_fields, is_active, display_order, endpoint_usage, created_at, updated_at) VALUES (UUID_TO_BIN(?), ?, 'turnstile', 'site-key', 'secret', ?, ?, ?, 'all', ?, ?)")
            .bind(Uuid::now_v7().to_string()).bind(name).bind(serde_json::json!(["hostname"]))
            .bind(active).bind(order).bind(time(day)).bind(time(day)).execute(&db.pool).await.unwrap();
    }
    let rows = repo.get_active_captcha_configs().await.unwrap();
    assert_eq!(
        rows.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
        ["first", "second", "last"]
    );
    assert_eq!(rows[0].capture_fields, ["hostname"]);
    assert_eq!(rows[0].site_key, "site-key");
    assert_eq!(rows[0].secret, "secret");
    assert!(rows[0].base_url.is_none());
    assert!(rows[0].verification.is_none());
    assert!(rows[0].endpoint_usage.matches_auth_code());
    assert!(rows[0].endpoint_usage.matches_reauth());
    assert!(
        rows[0]
            .widget
            .script_url
            .contains("challenges.cloudflare.com")
    );

    // Invalid JSON config is skipped without disabling otherwise valid providers.
    sqlx::query("UPDATE captcha_configs SET provider = 'custom', widget_form_field_name = 'answer', widget_script_url = '/widget.js', widget_html = '', base_url = 'https://captcha.example', verification = ?, capture_fields = NULL WHERE name = 'second'")
        .bind(serde_json::json!({"url":"{{base_url}}/verify"})).execute(&db.pool).await.unwrap();
    let rows = repo.get_active_captcha_configs().await.unwrap();
    assert_eq!(rows[1].widget.form_field_name, "answer");
    assert!(rows[1].capture_fields.is_empty());
    assert!(rows[1].verification.is_some());
    sqlx::query("UPDATE captcha_configs SET verification = ? WHERE name = 'second'")
        .bind(serde_json::json!({"headers":"invalid"}))
        .execute(&db.pool)
        .await
        .unwrap();
    assert_eq!(repo.get_active_captcha_configs().await.unwrap().len(), 2);
    sqlx::query("UPDATE captcha_configs SET secret = ''")
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(repo.get_active_captcha_configs().await.is_err());
    sqlx::query("UPDATE captcha_configs SET is_active = FALSE")
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(repo.get_active_captcha_configs().await.unwrap().is_empty());
    db.pool.close().await;
    assert!(repo.get_active_captcha_configs().await.is_err());
}
