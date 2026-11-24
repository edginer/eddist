//! Shared disposable-MySQL setup and inputs for repository integration tests and benchmarks.
use chrono::{NaiveDate, NaiveDateTime, Timelike};
use eddist::repositories::{
    bbs_repository::{BbsRepositoryImpl, CreatingAuthedToken, CreatingThread},
    user_repository::{CreatingUser, UserRepository, UserRepositoryImpl},
};
use eddist_core::domain::{
    client_info::ClientInfo, ip_addr::IpAddr, metadent::MetadentType,
    pubsub_repository::CreatingRes,
};
use sqlx::{MySqlPool, mysql::MySqlPoolOptions};
use std::time::Duration;
use testcontainers::{ContainerAsync, ImageExt, core::IntoContainerPort, runners::AsyncRunner};
use testcontainers_modules::mysql::Mysql;
use uuid::Uuid;

pub(crate) struct TestDatabase {
    pub(crate) pool: MySqlPool,
    _container: ContainerAsync<Mysql>,
}

pub(crate) type RecordedSql = std::sync::Arc<std::sync::Mutex<Vec<String>>>;

impl TestDatabase {
    pub(crate) fn users(&self) -> UserRepositoryImpl {
        UserRepositoryImpl::new(self.orm())
    }

    pub(crate) async fn register_user(
        &self,
        user: CreatingUser,
        token_id: Uuid,
    ) -> anyhow::Result<()> {
        self.users()
            .register_user_and_bind_token(user, token_id)
            .await
    }

    pub(crate) async fn bind_user(&self, user_id: Uuid, token_id: Uuid) -> anyhow::Result<()> {
        self.users().bind_user_authed_token(user_id, token_id).await
    }

    pub(crate) async fn insert_idp(&self, name: &str, enabled: bool) -> Uuid {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO idps (id, idp_name, idp_display_name, oidc_config_url, client_id, client_secret, enabled) VALUES (UUID_TO_BIN(?), ?, ?, 'https://idp.example/config', 'client', 'encrypted', ?)")
            .bind(id.to_string()).bind(name).bind(format!("認証 {name}")).bind(enabled).execute(&self.pool).await.unwrap();
        id
    }

    pub(crate) fn bbs_with_statements(&self) -> (BbsRepositoryImpl, RecordedSql) {
        let mut db = self.orm();
        let statements = RecordedSql::default();
        db.set_metric_callback({
            let statements = statements.clone();
            move |info| statements.lock().unwrap().push(info.statement.sql.clone())
        });
        (BbsRepositoryImpl::new(db), statements)
    }

    pub(crate) fn bbs(&self) -> BbsRepositoryImpl {
        BbsRepositoryImpl::new(self.orm())
    }

    pub(crate) async fn insert_board(&self, key: &str) -> Uuid {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO boards (id, name, board_key, default_name) VALUES (UUID_TO_BIN(?), ?, ?, ?)")
            .bind(id.to_string()).bind(format!("板 {key}")).bind(key).bind("名無さん")
            .execute(&self.pool).await.unwrap();
        id
    }

    pub(crate) async fn url(&self) -> String {
        let port = self
            ._container
            .get_host_port_ipv4(3306.tcp())
            .await
            .unwrap();
        format!("mysql://root@127.0.0.1:{port}/test")
    }

    pub(crate) fn orm(&self) -> sea_orm::DatabaseConnection {
        sea_orm::SqlxMySqlConnector::from_sqlx_mysql_pool(self.pool.clone())
    }

    pub(crate) async fn new() -> Self {
        Self::with_max_connections(1).await
    }

    pub(crate) async fn with_max_connections(max_connections: u32) -> Self {
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

pub(crate) fn time(day: u32) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2020, 1, day)
        .unwrap()
        .and_hms_milli_opt(12, 34, 56, 123)
        .unwrap()
}

pub(crate) fn token_fixture(name: &str, ip: &str, code: &str) -> CreatingAuthedToken {
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

pub(crate) fn user_fixture(idp_id: Uuid, name: &str) -> CreatingUser {
    CreatingUser {
        user_id: Uuid::now_v7(),
        user_name: name.into(),
        idp_id,
        idp_sub: "1".repeat(64),
    }
}

pub(crate) fn thread_fixture(board_id: Uuid, token_id: Uuid, number: u64) -> CreatingThread {
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

pub(crate) fn response_fixture(thread: &CreatingThread, order: i32) -> CreatingRes {
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
