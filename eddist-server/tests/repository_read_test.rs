//! Behavior checks shared by the SQLx baseline and the incremental SeaORM migration.
use chrono::{NaiveDate, NaiveDateTime, Timelike};
use eddist::repositories::{
    bbs_repository::{
        AuthedTokenRepository, BbsRepositoryImpl, BoardRepository, CreateThreadError,
        CreatingAuthedToken, CreatingThread, ResponseRepository, ThreadRepository,
    },
    captcha_config_repository::{CaptchaConfigRepository, CaptchaConfigRepositoryImpl},
    idp_repository::{IdpRepository, IdpRepositoryImpl},
    notice_repository::{NoticeRepository, NoticeRepositoryImpl},
    server_settings_repository::{ServerSettingsRepository, ServerSettingsRepositoryImpl},
    stats_repository::{StatsRepository, StatsRepositoryImpl},
    terms_repository::{TermsRepository, TermsRepositoryImpl},
    user_repository::{CreatingUser, UserRepository, UserRepositoryImpl},
    user_restriction_repository::{UserRestrictionRepository, UserRestrictionRepositoryImpl},
};
use eddist_core::{
    domain::{
        client_info::ClientInfo,
        ip_addr::IpAddr,
        metadent::MetadentType,
        pubsub_repository::CreatingRes,
        res::ResView,
        user_restriction::{RestrictionRuleType, RestrictionTarget},
    },
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

type RecordedSql = std::sync::Arc<std::sync::Mutex<Vec<String>>>;

impl TestDatabase {
    fn users(&self) -> UserRepositoryImpl {
        UserRepositoryImpl::new(self.orm())
    }

    async fn register_user(&self, user: CreatingUser, token_id: Uuid) -> anyhow::Result<()> {
        self.users()
            .register_user_and_bind_token(user, token_id)
            .await
    }

    async fn bind_user(&self, user_id: Uuid, token_id: Uuid) -> anyhow::Result<()> {
        self.users().bind_user_authed_token(user_id, token_id).await
    }

    async fn insert_idp(&self, name: &str, enabled: bool) -> Uuid {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO idps (id, idp_name, idp_display_name, oidc_config_url, client_id, client_secret, enabled) VALUES (UUID_TO_BIN(?), ?, ?, 'https://idp.example/config', 'client', 'encrypted', ?)")
            .bind(id.to_string()).bind(name).bind(format!("認証 {name}")).bind(enabled).execute(&self.pool).await.unwrap();
        id
    }

    fn bbs_with_statements(&self) -> (BbsRepositoryImpl, RecordedSql) {
        let mut db = self.orm();
        let statements = RecordedSql::default();
        db.set_metric_callback({
            let statements = statements.clone();
            move |info| statements.lock().unwrap().push(info.statement.sql.clone())
        });
        (BbsRepositoryImpl::new(self.pool.clone(), db), statements)
    }

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

fn token_fixture(name: &str, ip: &str, code: &str) -> CreatingAuthedToken {
    CreatingAuthedToken {
        id: Uuid::now_v7(),
        token: format!("token-{name}"),
        origin_ip: IpAddr::new(ip.to_owned()),
        asn_num: 64500,
        writing_ua: format!("UA {name}"),
        auth_code: code.to_owned(),
        created_at: time(1).with_nanosecond(999_600_000).unwrap().and_utc(),
        author_id_seed: (0..64).collect(),
        require_user_registration: name == "first",
    }
}

fn user_fixture(idp_id: Uuid, name: &str) -> CreatingUser {
    CreatingUser {
        user_id: Uuid::now_v7(),
        user_name: name.into(),
        idp_id,
        idp_sub: "1".repeat(64),
    }
}

fn thread_fixture(board_id: Uuid, token_id: Uuid, number: u64) -> CreatingThread {
    CreatingThread {
        thread_id: Uuid::now_v7(),
        response_id: Uuid::now_v7(),
        title: "日本語スレッド &amp; タイトル".into(),
        unix_time: number,
        body: "本文<br>２行目".into(),
        name: "投稿者 ◆test".into(),
        mail: "sage".into(),
        created_at: time(2).with_nanosecond(999_600_000).unwrap().and_utc(),
        author_ch5id: "AuthorID".into(),
        authed_token_id: token_id,
        ip_addr: "2001:db8::1234".into(),
        board_id,
        metadent: MetadentType::VVVerbose,
        client_info: ClientInfo {
            user_agent: "UA 日本語".into(),
            asn_num: 64501,
            ip_addr: "2001:db8::1234".into(),
            tinker: None,
        },
        moderation_result: None,
    }
}

fn response_fixture(thread: &CreatingThread, order: i32) -> CreatingRes {
    CreatingRes {
        id: Uuid::now_v7(),
        created_at: time(3).with_nanosecond(999_600_000).unwrap().and_utc(),
        body: format!("返信 {order}<br>日本語"),
        name: "".into(),
        mail: "sage".into(),
        author_ch5id: "ReplyID".into(),
        authed_token_id: thread.authed_token_id,
        ip_addr: "192.0.2.10".into(),
        thread_id: thread.thread_id,
        board_id: thread.board_id,
        client_info: thread.client_info.clone(),
        res_order: order,
        is_sage: true,
        moderation_result: None,
    }
}

async fn thread_write_state(
    db: &TestDatabase,
    id: Uuid,
) -> (i32, bool, NaiveDateTime, NaiveDateTime) {
    sqlx::query_as("SELECT response_count, active, last_modified_at, sage_last_modified_at FROM threads WHERE id = ?")
        .bind(id).fetch_one(&db.pool).await.unwrap()
}

fn assert_posting_statements(statements: &RecordedSql, expected: &[&str]) {
    let statements = statements.lock().unwrap();
    let verbs: Vec<_> = statements
        .iter()
        .filter_map(|sql| sql.split_whitespace().next())
        .filter(|verb| ["SELECT", "INSERT", "UPDATE", "DELETE"].contains(verb))
        .collect();
    assert_eq!(verbs, expected, "unexpected model reload or extra DML");
}

#[tokio::test]
async fn posting_thread_and_first_response_are_atomic_and_classify_conflicts() {
    let db = TestDatabase::new().await;
    let (repo, statements) = db.bbs_with_statements();
    let board_id = db.insert_board("posting").await;
    let token = token_fixture("posting", "192.0.2.1", "123456");
    repo.create_authed_token(token.clone()).await.unwrap();
    // The runtime pool will eventually stop setting TIME_TRUNCATE_FRACTIONAL.
    sqlx::query("SET SESSION sql_mode = REPLACE(@@sql_mode, ',TIME_TRUNCATE_FRACTIONAL', '')")
        .execute(&db.pool)
        .await
        .unwrap();
    let input = thread_fixture(board_id, token.id, 1_700_000_001);
    statements.lock().unwrap().clear();
    repo.create_thread(input.clone()).await.unwrap();
    assert_posting_statements(&statements, &["INSERT", "INSERT"]);
    let found = repo
        .get_thread_by_board_key_and_thread_number("posting", input.unix_time)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.id, input.thread_id);
    assert_eq!(found.board_id, board_id);
    assert_eq!(found.title, input.title);
    assert_eq!(found.authed_token_id, token.id);
    assert_eq!(found.metadent, "vvv");
    assert_eq!(found.response_count, 1);
    assert!(found.active);
    assert!(!found.archived);
    assert!(!found.no_pool);
    let stored_at = eddist_entity::db_time::truncate_to_millis(input.created_at.naive_utc());
    assert_eq!(found.last_modified_at.naive_utc(), stored_at);
    assert_eq!(found.sage_last_modified_at.naive_utc(), stored_at);
    let responses = repo.get_responses(input.thread_id).await.unwrap();
    assert_eq!(responses.len(), 1);
    let expected = ResView {
        author_name: input.name.clone(),
        mail: input.mail.clone(),
        body: input.body.clone(),
        created_at: stored_at.and_utc(),
        author_id: input.author_ch5id.clone(),
        is_abone: false,
        is_abone_keep_id: false,
    };
    assert_eq!(
        responses[0]
            .get_sjis_bytes("名無さん", Some(&input.title))
            .get_inner(),
        expected
            .get_sjis_bytes("名無さん", Some(&input.title))
            .get_inner()
    );
    let extra: (Uuid, Uuid, Uuid, String, serde_json::Value, i32) = sqlx::query_as("SELECT thread_id, board_id, authed_token_id, ip_addr, client_info, res_order FROM responses WHERE id = ?")
        .bind(input.response_id).fetch_one(&db.pool).await.unwrap();
    assert_eq!(
        extra,
        (
            input.thread_id,
            board_id,
            token.id,
            input.ip_addr.clone(),
            serde_json::to_value(&input.client_info).unwrap(),
            1
        )
    );

    let mut conflict = input.clone();
    conflict.thread_id = Uuid::now_v7();
    conflict.response_id = Uuid::now_v7();
    assert!(matches!(
        repo.create_thread(conflict.clone()).await,
        Err(CreateThreadError::ThreadNumberConflict)
    ));
    // The existing contract classifies every unique violation of the thread INSERT,
    // including its primary key; response INSERT errors remain Other.
    conflict.thread_id = input.thread_id;
    conflict.unix_time += 1;
    assert!(matches!(
        repo.create_thread(conflict).await,
        Err(CreateThreadError::ThreadNumberConflict)
    ));
    let mut failed = thread_fixture(board_id, token.id, input.unix_time + 2);
    failed.response_id = input.response_id;
    assert!(matches!(
        repo.create_thread(failed.clone()).await,
        Err(CreateThreadError::Other(_))
    ));
    assert!(
        repo.get_thread_by_board_key_and_thread_number("posting", failed.unix_time)
            .await
            .unwrap()
            .is_none()
    );
    for (bad_board, bad_token) in [(Uuid::nil(), token.id), (board_id, Uuid::nil())] {
        let failed = thread_fixture(bad_board, bad_token, input.unix_time + 3);
        assert!(matches!(
            repo.create_thread(failed).await,
            Err(CreateThreadError::Other(_))
        ));
    }
    let other_board = db.insert_board("posting-other").await;
    let mut other = thread_fixture(other_board, token.id, input.unix_time);
    other.metadent = MetadentType::None;
    repo.create_thread(other.clone()).await.unwrap();
    assert_eq!(
        repo.get_thread_by_board_key_and_thread_number("posting-other", other.unix_time)
            .await
            .unwrap()
            .unwrap()
            .metadent,
        ""
    );
    let counts: (i64, i64) =
        sqlx::query_as("SELECT (SELECT COUNT(*) FROM threads), (SELECT COUNT(*) FROM responses)")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(counts, (2, 2));
    db.pool.close().await;
    assert!(matches!(
        repo.create_thread(thread_fixture(board_id, token.id, 999))
            .await,
        Err(CreateThreadError::Other(_))
    ));
}

#[tokio::test]
async fn posting_response_preserves_count_boundary_sage_precision_and_rollback() {
    let db = TestDatabase::new().await;
    let (repo, statements) = db.bbs_with_statements();
    let board_id = db.insert_board("reply").await;
    let token = token_fixture("reply", "192.0.2.1", "123456");
    repo.create_authed_token(token.clone()).await.unwrap();
    let thread = thread_fixture(board_id, token.id, 1_700_000_002);
    repo.create_thread(thread.clone()).await.unwrap();
    sqlx::query("SET SESSION sql_mode = REPLACE(@@sql_mode, ',TIME_TRUNCATE_FRACTIONAL', '')")
        .execute(&db.pool)
        .await
        .unwrap();
    for (count, sage) in [(998, true), (999, false), (1000, true)] {
        sqlx::query("UPDATE threads SET response_count = ?, active = FALSE, archived = TRUE, last_modified_at = ?, sage_last_modified_at = ? WHERE id = ?")
            .bind(count).bind(time(1)).bind(time(1)).bind(thread.thread_id).execute(&db.pool).await.unwrap();
        let mut res = response_fixture(&thread, count + 1);
        res.is_sage = sage;
        res.mail = if sage { "sage" } else { "email@example.com" }.into();
        statements.lock().unwrap().clear();
        repo.create_response(res.clone()).await.unwrap();
        assert_posting_statements(&statements, &["UPDATE", "INSERT"]);
        let stored_at = eddist_entity::db_time::truncate_to_millis(res.created_at.naive_utc());
        assert_eq!(
            thread_write_state(&db, thread.thread_id).await,
            (
                count + 1,
                count + 1 < 1000,
                stored_at,
                if sage { time(1) } else { stored_at }
            )
        );
        let responses = repo.get_responses(thread.thread_id).await.unwrap();
        let found = responses.last().unwrap();
        assert_eq!(found.created_at.naive_utc(), stored_at);
        let expected = ResView {
            author_name: res.name,
            mail: res.mail,
            body: res.body,
            created_at: stored_at.and_utc(),
            author_id: res.author_ch5id,
            is_abone: false,
            is_abone_keep_id: false,
        };
        assert_eq!(
            found.get_sjis_bytes("名無さん", None).get_inner(),
            expected.get_sjis_bytes("名無さん", None).get_inner()
        );
    }
    let before = thread_write_state(&db, thread.thread_id).await;
    for (res_id, board_id, thread_id) in [
        (thread.response_id, board_id, thread.thread_id),
        (Uuid::now_v7(), Uuid::nil(), thread.thread_id),
        (Uuid::now_v7(), board_id, Uuid::nil()),
    ] {
        let mut res = response_fixture(&thread, 1002);
        res.id = res_id;
        res.board_id = board_id;
        res.thread_id = thread_id;
        res.is_sage = false;
        res.created_at = time(4).and_utc();
        assert!(repo.create_response(res).await.is_err());
        assert_eq!(thread_write_state(&db, thread.thread_id).await, before);
        assert_eq!(repo.get_responses(thread.thread_id).await.unwrap().len(), 4);
    }
    db.pool.close().await;
    assert!(
        repo.create_response(response_fixture(&thread, 1002))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn posting_parallel_responses_increment_without_lost_updates() {
    let db = TestDatabase::with_max_connections(4).await;
    let repo = db.bbs();
    let board_id = db.insert_board("parallel-reply").await;
    let token = token_fixture("parallel-reply", "192.0.2.1", "123456");
    repo.create_authed_token(token.clone()).await.unwrap();
    let thread = thread_fixture(board_id, token.id, 1_700_000_003);
    repo.create_thread(thread.clone()).await.unwrap();
    sqlx::query("UPDATE threads SET response_count = 998 WHERE id = ?")
        .bind(thread.thread_id)
        .execute(&db.pool)
        .await
        .unwrap();
    let gate = std::sync::Arc::new(tokio::sync::Barrier::new(9));
    let mut jobs = tokio::task::JoinSet::new();
    for order in 999..1007 {
        let repo = repo.clone();
        let gate = gate.clone();
        let res = response_fixture(&thread, order);
        jobs.spawn(async move {
            gate.wait().await;
            repo.create_response(res).await
        });
    }
    gate.wait().await;
    while let Some(result) = jobs.join_next().await {
        result.unwrap().unwrap();
    }
    let (count, active, _, _) = thread_write_state(&db, thread.thread_id).await;
    assert_eq!(count, 1006);
    assert!(!active);
    assert_eq!(repo.get_responses(thread.thread_id).await.unwrap().len(), 9);
}

#[tokio::test]
async fn user_reads_preserve_join_projection_disabled_state_and_token_order() {
    let db = TestDatabase::new().await;
    let repo = db.users();
    let bbs = db.bbs();
    let idp = db.insert_idp("first", true).await;
    let other_idp = db.insert_idp("second", false).await;
    let user = user_fixture(idp, "利用者");
    let mut tokens = Vec::new();
    for name in ["userfirst", "usersecond", "userthird", "unbound"] {
        let token = token_fixture(name, "192.0.2.1", name);
        bbs.create_authed_token(token.clone()).await.unwrap();
        tokens.push(token);
    }
    db.register_user(user.clone(), tokens[0].id).await.unwrap();
    sqlx::query("UPDATE users SET enabled = FALSE, created_at = ?, updated_at = ? WHERE id = UUID_TO_BIN(?)")
        .bind(time(1)).bind(time(2)).bind(user.user_id.to_string()).execute(&db.pool).await.unwrap();
    sqlx::query("UPDATE user_idp_bindings SET created_at = ?, updated_at = ? WHERE user_id = UUID_TO_BIN(?)")
        .bind(time(3)).bind(time(4)).bind(user.user_id.to_string()).execute(&db.pool).await.unwrap();
    sqlx::query("INSERT INTO user_idp_bindings (id, user_id, idp_id, idp_sub, created_at, updated_at) VALUES (UUID_TO_BIN(?), UUID_TO_BIN(?), UUID_TO_BIN(?), ?, ?, ?)")
        .bind(Uuid::now_v7().to_string()).bind(user.user_id.to_string()).bind(other_idp.to_string()).bind("2".repeat(64)).bind(time(2)).bind(time(3)).execute(&db.pool).await.unwrap();
    let mut found = repo.get_user_by_id(user.user_id).await.unwrap().unwrap();
    assert_eq!(found.id, user.user_id);
    assert_eq!(found.user_name, "利用者");
    assert!(!found.enabled);
    assert_eq!(found.created_at, time(1));
    assert_eq!(found.updated_at, time(2));
    found.idps.sort_by(|a, b| a.idp_name.cmp(&b.idp_name));
    assert_eq!(found.idps.len(), 2);
    assert_eq!(found.idps[0].idp_id, idp);
    assert_eq!(found.idps[0].idp_name, "first");
    assert_eq!(found.idps[0].idp_display_name, "認証 first");
    assert_eq!(found.idps[0].idp_sub, user.idp_sub);
    assert_eq!(found.idps[0].created_at, time(3));
    assert_eq!(found.idps[0].updated_at, time(4));
    assert_eq!(found.idps[1].idp_id, other_idp);
    let by_sub = repo
        .get_user_by_idp_sub("first", &user.idp_sub)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(by_sub.id, user.user_id);
    assert_eq!(by_sub.idps.len(), 1); // The WHERE clause selects only the matching IdP.
    assert!(!by_sub.enabled);
    assert!(repo.get_user_by_id(Uuid::nil()).await.unwrap().is_none());
    assert!(
        repo.get_user_by_idp_sub("second", &user.idp_sub)
            .await
            .unwrap()
            .is_none()
    );
    let orphan = Uuid::now_v7();
    sqlx::query("INSERT INTO users (id, user_name, created_at, updated_at) VALUES (UUID_TO_BIN(?), 'no IdP', ?, ?)")
        .bind(orphan.to_string()).bind(time(1)).bind(time(1)).execute(&db.pool).await.unwrap();
    assert!(repo.get_user_by_id(orphan).await.unwrap().is_none());
    for (index, token) in tokens[..3].iter().enumerate() {
        if index > 0 {
            db.bind_user(user.user_id, token.id).await.unwrap();
        }
        sqlx::query(
            "UPDATE user_authed_tokens SET created_at = ? WHERE authed_token_id = UUID_TO_BIN(?)",
        )
        .bind(time(index as u32 + 1))
        .bind(token.id.to_string())
        .execute(&db.pool)
        .await
        .unwrap();
    }
    assert_eq!(
        repo.get_all_authed_tokens_by_user_id(user.user_id)
            .await
            .unwrap(),
        tokens[..3].iter().map(|t| t.id).collect::<Vec<_>>()
    );
    sqlx::query("UPDATE authed_tokens SET validity = TRUE WHERE id = UUID_TO_BIN(?)")
        .bind(tokens[0].id.to_string())
        .execute(&db.pool)
        .await
        .unwrap();
    bbs.activate_authed_status(&tokens[1].token, "UA", time(2).and_utc(), None)
        .await
        .unwrap();
    sqlx::query("UPDATE authed_tokens SET authed_at = ? WHERE id = UUID_TO_BIN(?)")
        .bind(time(3))
        .bind(tokens[2].id.to_string())
        .execute(&db.pool)
        .await
        .unwrap();
    assert_eq!(
        repo.get_valid_authed_token_by_user_id(user.user_id)
            .await
            .unwrap(),
        Some(tokens[1].token.clone())
    );
    bbs.revoke_authed_token(&tokens[1].token).await.unwrap();
    assert_eq!(
        repo.get_valid_authed_token_by_user_id(user.user_id)
            .await
            .unwrap(),
        Some(tokens[0].token.clone())
    );
    bbs.revoke_authed_token(&tokens[0].token).await.unwrap();
    assert!(
        repo.get_valid_authed_token_by_user_id(user.user_id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        repo.get_all_authed_tokens_by_user_id(orphan)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        repo.get_valid_authed_token_by_user_id(orphan)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        repo.is_user_binded_authed_token(tokens[0].id)
            .await
            .unwrap()
    );
    assert!(
        !repo
            .is_user_binded_authed_token(tokens[3].id)
            .await
            .unwrap()
    );
    assert!(repo.is_user_binded_authed_token(Uuid::nil()).await.is_err());
    db.pool.close().await;
    assert!(repo.get_user_by_id(user.user_id).await.is_err());
    assert!(
        repo.get_user_by_idp_sub("first", &user.idp_sub)
            .await
            .is_err()
    );
    assert!(
        repo.get_all_authed_tokens_by_user_id(user.user_id)
            .await
            .is_err()
    );
    assert!(
        repo.get_valid_authed_token_by_user_id(user.user_id)
            .await
            .is_err()
    );
    assert!(
        repo.is_user_binded_authed_token(tokens[0].id)
            .await
            .is_err()
    );
    assert!(db.register_user(user, tokens[3].id).await.is_err());
    assert!(db.bind_user(orphan, tokens[3].id).await.is_err());
}

#[tokio::test]
async fn user_registration_is_atomic_and_keeps_database_second_precision() {
    let db = TestDatabase::new().await;
    let idp = db.insert_idp("first", true).await;
    let token = token_fixture("registration", "192.0.2.1", "code");
    db.bbs().create_authed_token(token.clone()).await.unwrap();
    let user = user_fixture(idp, "registered");
    let now = time(2).with_nanosecond(0).unwrap();
    sqlx::query("SET timestamp = ?")
        .bind(now.and_utc().timestamp())
        .execute(&db.pool)
        .await
        .unwrap();
    let mut orm = db.orm();
    let statements = RecordedSql::default();
    orm.set_metric_callback({
        let statements = statements.clone();
        move |info| statements.lock().unwrap().push(info.statement.sql.clone())
    });
    UserRepositoryImpl::new(orm)
        .register_user_and_bind_token(user.clone(), token.id)
        .await
        .unwrap();
    {
        let sql = statements.lock().unwrap();
        assert_eq!(sql.len(), 6); // Four DMLs plus the user lock and canonical seed SELECTs.
        assert_eq!(
            sql.iter().filter(|sql| sql.starts_with("SELECT")).count(),
            2
        );
    }
    let found = db
        .users()
        .get_user_by_id(user.user_id)
        .await
        .unwrap()
        .unwrap();
    assert!(found.enabled);
    assert_eq!(found.created_at, now);
    assert_eq!(found.updated_at, now);
    assert_eq!(found.idps[0].created_at, now);
    assert_eq!(found.idps[0].updated_at, now);
    let bound_at: NaiveDateTime = sqlx::query_scalar(
        "SELECT created_at FROM user_authed_tokens WHERE user_id = UUID_TO_BIN(?)",
    )
    .bind(user.user_id.to_string())
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(bound_at, now);
    let bound = db
        .bbs()
        .get_authed_token_by_id(token.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(bound.registered_user_id, Some(user.user_id));
    assert_eq!(bound.author_id_seed, token.author_id_seed);
    for (idp_id, token_id) in [(Uuid::nil(), token.id), (idp, Uuid::nil())] {
        let failing = user_fixture(idp_id, "must roll back");
        assert!(db.register_user(failing.clone(), token_id).await.is_err());
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE id = UUID_TO_BIN(?)")
            .bind(failing.user_id.to_string())
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM user_idp_bindings WHERE user_id = UUID_TO_BIN(?)",
        )
        .bind(failing.user_id.to_string())
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert_eq!(count, 0);
    }
    assert!(db.register_user(user.clone(), token.id).await.is_err());
    assert!(db.bind_user(user.user_id, token.id).await.is_err());
    assert_eq!(
        db.bbs()
            .get_authed_token_by_id(token.id)
            .await
            .unwrap()
            .unwrap()
            .author_id_seed,
        token.author_id_seed
    );
}

#[tokio::test]
async fn user_binding_serializes_first_tokens_and_inherits_the_canonical_seed() {
    let db = TestDatabase::with_max_connections(4).await;
    // The repository must set READ COMMITTED per transaction, independent of pool defaults.
    let mut connections = Vec::new();
    for _ in 0..4 {
        connections.push(db.pool.acquire().await.unwrap());
    }
    for connection in &mut connections {
        sqlx::query("SET SESSION TRANSACTION ISOLATION LEVEL REPEATABLE READ")
            .execute(&mut **connection)
            .await
            .unwrap();
    }
    drop(connections);
    let idp = db.insert_idp("first", true).await;
    let first = token_fixture("seedfirst", "192.0.2.1", "first");
    let mut second = token_fixture("seedsecond", "192.0.2.2", "second");
    second.author_id_seed = vec![99; 64];
    for token in [&first, &second] {
        db.bbs().create_authed_token(token.clone()).await.unwrap();
    }
    let existing = user_fixture(idp, "existing");
    db.register_user(existing.clone(), first.id).await.unwrap();
    sqlx::query("UPDATE user_authed_tokens SET created_at = ? WHERE user_id = UUID_TO_BIN(?)")
        .bind(time(1))
        .bind(existing.user_id.to_string())
        .execute(&db.pool)
        .await
        .unwrap();
    db.bind_user(existing.user_id, second.id).await.unwrap();
    assert_eq!(
        db.bbs()
            .get_authed_token_by_id(second.id)
            .await
            .unwrap()
            .unwrap()
            .author_id_seed,
        first.author_id_seed
    );
    assert!(db.bind_user(Uuid::nil(), second.id).await.is_err());
    assert!(db.bind_user(existing.user_id, Uuid::nil()).await.is_err());

    let user_id = Uuid::now_v7();
    sqlx::query("INSERT INTO users (id, user_name, created_at, updated_at) VALUES (UUID_TO_BIN(?), 'concurrent', ?, ?)")
        .bind(user_id.to_string()).bind(time(1)).bind(time(1)).execute(&db.pool).await.unwrap();
    let mut left = token_fixture("left", "192.0.2.3", "left");
    let mut right = token_fixture("right", "192.0.2.4", "right");
    left.author_id_seed = vec![11; 64];
    right.author_id_seed = vec![22; 64];
    for token in [&left, &right] {
        db.bbs().create_authed_token(token.clone()).await.unwrap();
    }
    let mut gate = db.pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM users WHERE id = UUID_TO_BIN(?) FOR UPDATE")
        .bind(user_id.to_string())
        .fetch_one(&mut *gate)
        .await
        .unwrap();
    let observe_and_release = async {
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let modes: Vec<String> = sqlx::query_scalar("SELECT LOCK_MODE FROM performance_schema.data_locks WHERE OBJECT_NAME = 'users' AND LOCK_STATUS = 'WAITING'")
                    .fetch_all(&db.pool).await.unwrap();
                let levels: Vec<String> = sqlx::query_scalar("SELECT trx_isolation_level FROM information_schema.innodb_trx WHERE trx_state = 'LOCK WAIT'")
                    .fetch_all(&db.pool).await.unwrap();
                if modes.len() >= 2 && levels.len() >= 2 {
                    assert!(modes.iter().all(|mode| mode.starts_with('X')));
                    assert!(levels.iter().all(|level| level == "READ COMMITTED"));
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }).await.expect("both bindings must wait for an exclusive user row lock");
        gate.commit().await.unwrap();
    };
    let (left_result, right_result, ()) = tokio::join!(
        db.bind_user(user_id, left.id),
        db.bind_user(user_id, right.id),
        observe_and_release
    );
    left_result.unwrap();
    right_result.unwrap();
    let left_bound = db
        .bbs()
        .get_authed_token_by_id(left.id)
        .await
        .unwrap()
        .unwrap();
    let right_bound = db
        .bbs()
        .get_authed_token_by_id(right.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(left_bound.registered_user_id, Some(user_id));
    assert_eq!(right_bound.registered_user_id, Some(user_id));
    assert_eq!(left_bound.author_id_seed, right_bound.author_id_seed);
    assert!(
        left_bound.author_id_seed == left.author_id_seed
            || left_bound.author_id_seed == right.author_id_seed
    );
    assert_eq!(
        db.users()
            .get_all_authed_tokens_by_user_id(user_id)
            .await
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn token_reads_preserve_filters_all_fields_and_nullable_values() {
    let db = TestDatabase::new().await;
    let (repo, statements) = db.bbs_with_statements();
    let first = token_fixture("first", "192.0.2.1", "shared");
    let second = token_fixture("second", "2001:db8:abcd:1234::1", "shared");
    let other = token_fixture("other", "192.0.2.1", "other");
    for token in [&first, &second, &other] {
        repo.create_authed_token(token.clone()).await.unwrap();
    }
    let row = repo.get_authed_token(&first.token).await.unwrap().unwrap();
    assert_eq!(row.id, first.id);
    assert_eq!(row.token, first.token);
    assert_eq!(row.origin_ip.to_string(), "192.0.2.1");
    assert_eq!(row.reduced_ip.to_string(), "192.0.2.1");
    assert_eq!(row.asn_num, 64500);
    assert_eq!(row.writing_ua, "UA first");
    assert_eq!(row.auth_code, "shared");
    assert_eq!(row.created_at.timestamp(), first.created_at.timestamp());
    assert_eq!(row.created_at.timestamp_subsec_nanos(), 999_000_000);
    assert!(row.authed_ua.is_none());
    assert!(row.authed_at.is_none());
    assert!(!row.validity);
    assert!(row.last_wrote_at.is_none());
    assert_eq!(row.author_id_seed, first.author_id_seed);
    assert!(row.require_user_registration);
    assert!(row.registered_user_id.is_none());
    assert!(!row.require_reauth);
    let row = repo
        .get_authed_token_by_id(second.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.origin_ip.to_string(), "2001:db8:abcd:1234::1");
    assert_eq!(row.reduced_ip.to_string(), "2001:db8:abcd:1234");
    assert!(!row.require_user_registration);
    assert_eq!(
        repo.get_authed_token_by_origin_ip_and_auth_code("2001:db8:abcd:1234", "shared")
            .await
            .unwrap()
            .unwrap()
            .id,
        second.id
    );
    assert!(
        repo.get_authed_token_by_origin_ip_and_auth_code("2001:db8:abcd:1234::1", "shared")
            .await
            .unwrap()
            .is_none()
    );
    assert!(repo.get_authed_token("missing").await.unwrap().is_none());
    assert!(
        repo.get_authed_token_by_id(Uuid::nil())
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        repo.get_authed_token_by_origin_ip_and_auth_code("192.0.2.99", "shared")
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        repo.get_authed_token_by_origin_ip_and_auth_code("192.0.2.1", "missing")
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        repo.get_unauthed_authed_token_by_auth_code("missing")
            .await
            .unwrap()
            .is_empty()
    );
    let mut ids: Vec<_> = repo
        .get_unauthed_authed_token_by_auth_code("shared")
        .await
        .unwrap()
        .into_iter()
        .map(|t| t.id)
        .collect();
    ids.sort();
    let mut expected = vec![first.id, second.id];
    expected.sort();
    assert_eq!(ids, expected);

    let user_id = Uuid::now_v7();
    sqlx::query("INSERT INTO users (id, user_name, enabled, created_at, updated_at) VALUES (UUID_TO_BIN(?), 'user', TRUE, ?, ?)")
        .bind(user_id.to_string()).bind(time(1)).bind(time(1)).execute(&db.pool).await.unwrap();
    sqlx::query("UPDATE authed_tokens SET validity = TRUE, registered_user_id = UUID_TO_BIN(?), require_reauth = TRUE, authed_ua = 'activated UA', authed_at = ?, last_wrote_at = ?, additional_info = JSON_OBJECT('sample', REPEAT('x', 2048)) WHERE id = UUID_TO_BIN(?)")
        .bind(user_id.to_string()).bind(time(2)).bind(time(3)).bind(first.id.to_string()).execute(&db.pool).await.unwrap();
    let row = repo
        .get_authed_token_by_id(first.id)
        .await
        .unwrap()
        .unwrap();
    assert!(row.validity);
    assert_eq!(row.registered_user_id, Some(user_id));
    assert!(row.require_reauth);
    assert!(row.require_user_registration);
    assert_eq!(row.authed_ua.as_deref(), Some("activated UA"));
    assert_eq!(row.authed_at, Some(time(2).and_utc()));
    assert_eq!(row.last_wrote_at, Some(time(3).and_utc()));
    assert_eq!(row.author_id_seed, first.author_id_seed);
    // This lookup does not filter validity; only the unauthed-list lookup does.
    assert_eq!(
        repo.get_authed_token_by_origin_ip_and_auth_code("192.0.2.1", "shared")
            .await
            .unwrap()
            .unwrap()
            .id,
        first.id
    );
    let rows = repo
        .get_unauthed_authed_token_by_auth_code("shared")
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, second.id);
    let sql = statements.lock().unwrap();
    let selects: Vec<_> = sql.iter().filter(|sql| sql.starts_with("SELECT")).collect();
    assert!(!selects.is_empty());
    assert!(selects.iter().all(|sql| !sql.contains("additional_info")));
}

#[tokio::test]
async fn token_lifecycle_preserves_json_null_precision_and_noop_updates() {
    let db = TestDatabase::new().await;
    let (repo, statements) = db.bbs_with_statements();
    // The repository must truncate before binding, even when MySQL would round.
    sqlx::query("SET SESSION sql_mode = TRIM(BOTH ',' FROM REPLACE(@@sql_mode, 'TIME_TRUNCATE_FRACTIONAL', ''))")
        .execute(&db.pool).await.unwrap();
    let token = token_fixture("lifecycle", "::1", "code");
    repo.create_authed_token(token.clone()).await.unwrap();
    let authed_at = time(2).with_nanosecond(999_600_000).unwrap().and_utc();
    for value in [
        None,
        Some(serde_json::Value::Null),
        Some(serde_json::json!({"日本語": [1, true, null]})),
        None,
    ] {
        repo.activate_authed_status(&token.token, "activated UA", authed_at, value.clone())
            .await
            .unwrap();
        let stored: Option<sqlx::types::Json<serde_json::Value>> = sqlx::query_scalar(
            "SELECT additional_info FROM authed_tokens WHERE id = UUID_TO_BIN(?)",
        )
        .bind(token.id.to_string())
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert_eq!(stored.map(|v| v.0), value);
    }
    let wrote_at = time(3).with_nanosecond(999_600_000).unwrap().and_utc();
    repo.update_authed_token_last_wrote(token.id, wrote_at)
        .await
        .unwrap();
    sqlx::query("UPDATE authed_tokens SET require_reauth = TRUE WHERE id = UUID_TO_BIN(?)")
        .bind(token.id.to_string())
        .execute(&db.pool)
        .await
        .unwrap();
    repo.clear_require_reauth(token.id).await.unwrap();
    let row = repo
        .get_authed_token_by_id(token.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.created_at.timestamp(), token.created_at.timestamp());
    assert_eq!(row.created_at.timestamp_subsec_nanos(), 999_000_000);
    assert!(row.validity);
    assert!(!row.require_reauth);
    assert_eq!(row.authed_ua.as_deref(), Some("activated UA"));
    assert_eq!(row.authed_at.unwrap().timestamp(), authed_at.timestamp());
    assert_eq!(row.authed_at.unwrap().timestamp_subsec_nanos(), 999_000_000);
    assert_eq!(row.last_wrote_at.unwrap().timestamp(), wrote_at.timestamp());
    assert_eq!(
        row.last_wrote_at.unwrap().timestamp_subsec_nanos(),
        999_000_000
    );
    repo.revoke_authed_token(&token.token).await.unwrap();
    let row = repo.get_authed_token(&token.token).await.unwrap().unwrap();
    assert!(!row.validity);
    assert_eq!(row.authed_ua.as_deref(), Some("activated UA"));
    assert!(row.authed_at.is_some());
    assert!(row.last_wrote_at.is_some());
    repo.delete_authed_token(&token.token).await.unwrap();
    assert!(repo.get_authed_token(&token.token).await.unwrap().is_none());
    // The SQLx API treats missing rows as successful no-ops for every mutation.
    repo.delete_authed_token(&token.token).await.unwrap();
    repo.revoke_authed_token(&token.token).await.unwrap();
    repo.activate_authed_status(&token.token, "UA", authed_at, None)
        .await
        .unwrap();
    repo.update_authed_token_last_wrote(token.id, wrote_at)
        .await
        .unwrap();
    repo.clear_require_reauth(token.id).await.unwrap();
    let sql = statements.lock().unwrap();
    // Fourteen explicit mutations and three explicit reads; no model reloads.
    assert_eq!(sql.len(), 17);
    assert_eq!(
        sql.iter().filter(|sql| sql.starts_with("SELECT")).count(),
        3
    );
}

#[tokio::test]
async fn token_unique_conflicts_and_database_failures_are_errors() {
    let db = TestDatabase::new().await;
    let repo = db.bbs();
    let token = token_fixture("errors", "192.0.2.1", "code");
    repo.create_authed_token(token.clone()).await.unwrap();
    let mut duplicate = token.clone();
    duplicate.id = Uuid::now_v7();
    duplicate.writing_ua = "must not overwrite".into();
    assert!(repo.create_authed_token(duplicate).await.is_err());
    assert_eq!(
        repo.get_authed_token(&token.token)
            .await
            .unwrap()
            .unwrap()
            .writing_ua,
        token.writing_ua
    );
    db.pool.close().await;
    assert!(repo.get_authed_token(&token.token).await.is_err());
    assert!(repo.get_authed_token_by_id(token.id).await.is_err());
    assert!(
        repo.get_authed_token_by_origin_ip_and_auth_code("192.0.2.1", "code")
            .await
            .is_err()
    );
    assert!(
        repo.get_unauthed_authed_token_by_auth_code("code")
            .await
            .is_err()
    );
    assert!(repo.create_authed_token(token.clone()).await.is_err());
    assert!(
        repo.activate_authed_status(&token.token, "UA", time(1).and_utc(), None)
            .await
            .is_err()
    );
    assert!(
        repo.update_authed_token_last_wrote(token.id, time(1).and_utc())
            .await
            .is_err()
    );
    assert!(repo.revoke_authed_token(&token.token).await.is_err());
    assert!(repo.delete_authed_token(&token.token).await.is_err());
    assert!(repo.clear_require_reauth(token.id).await.is_err());
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
