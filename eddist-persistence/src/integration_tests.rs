use chrono::{DateTime, NaiveDateTime, Utc};
use eddist_core::domain::{client_info::ClientInfo, pubsub_repository::CreatingRes};
use eddist_entity::{authed_token, board, response, thread};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, Database, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter,
};
use testcontainers::{ContainerAsync, ImageExt, core::IntoContainerPort, runners::AsyncRunner};
use testcontainers_modules::mysql::Mysql;
use uuid::Uuid;

use crate::persistence::insert_multiple_res;

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

struct Fixture {
    board_id: Uuid,
    authed_token_id: Uuid,
}

fn timestamp(s: &str) -> DateTime<Utc> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f")
        .unwrap()
        .and_utc()
}

async fn insert_fixture(db: &DatabaseConnection) -> anyhow::Result<Fixture> {
    let board_id = Uuid::now_v7();
    board::ActiveModel {
        id: Set(board_id),
        name: Set("board".to_string()),
        board_key: Set("board".to_string()),
        default_name: Set("名無し".to_string()),
    }
    .insert(db)
    .await?;

    let authed_token_id = Uuid::now_v7();
    let created_at = timestamp("2026-01-01 00:00:00");
    authed_token::ActiveModel {
        id: Set(authed_token_id),
        token: Set(authed_token_id.simple().to_string()),
        origin_ip: Set("192.0.2.1".to_string()),
        reduced_origin_ip: Set("192.0.2.0".to_string()),
        writing_ua: Set("test-ua".to_string()),
        authed_ua: Set(Some("test-ua".to_string())),
        auth_code: Set("000000".to_string()),
        created_at: Set(created_at),
        authed_at: Set(Some(created_at)),
        validity: Set(true),
        last_wrote_at: Set(None),
        asn_num: Set(0),
        additional_info: Set(None),
        require_user_registration: Set(false),
        registered_user_id: Set(None),
        require_reauth: Set(false),
        author_id_seed: Set(vec![0; 64]),
    }
    .insert(db)
    .await?;

    Ok(Fixture {
        board_id,
        authed_token_id,
    })
}

async fn insert_thread(
    db: &DatabaseConnection,
    fixture: &Fixture,
    thread_number: i64,
    last_modified_at: DateTime<Utc>,
    archived: bool,
) -> anyhow::Result<Uuid> {
    let id = Uuid::now_v7();
    thread::ActiveModel {
        id: Set(id),
        board_id: Set(fixture.board_id),
        thread_number: Set(thread_number),
        last_modified_at: Set(last_modified_at),
        sage_last_modified_at: Set(last_modified_at),
        title: Set("title".to_string()),
        authed_token_id: Set(fixture.authed_token_id),
        metadent: Set(String::new()),
        response_count: Set(0),
        no_pool: Set(false),
        active: Set(!archived),
        archived: Set(archived),
        archive_converted: Set(false),
    }
    .insert(db)
    .await?;
    Ok(id)
}

fn creating_res(
    fixture: &Fixture,
    thread_id: Uuid,
    res_order: i32,
    created_at: DateTime<Utc>,
) -> CreatingRes {
    CreatingRes {
        id: Uuid::now_v7(),
        created_at,
        body: "body".to_string(),
        name: "名無し".to_string(),
        mail: String::new(),
        author_ch5id: "abcdefgh".to_string(),
        authed_token_id: fixture.authed_token_id,
        ip_addr: "192.0.2.1".to_string(),
        thread_id,
        board_id: fixture.board_id,
        client_info: ClientInfo {
            user_agent: "test-ua".to_string(),
            asn_num: 0,
            ip_addr: "192.0.2.1".to_string(),
            tinker: None,
        },
        res_order,
        is_sage: false,
        moderation_result: None,
    }
}

fn utc(s: &str) -> DateTime<Utc> {
    timestamp(s)
}

async fn response_count(db: &DatabaseConnection, thread_id: Uuid) -> anyhow::Result<u64> {
    Ok(response::Entity::find()
        .filter(response::Column::ThreadId.eq(thread_id))
        .count(db)
        .await?)
}

#[tokio::test]
async fn replaying_cached_responses_ignores_duplicates() -> anyhow::Result<()> {
    let test_db = setup_database().await?;
    let db = &test_db.db;
    let fixture = insert_fixture(db).await?;
    let thread_id = insert_thread(db, &fixture, 1, timestamp("2026-01-01 00:00:00"), false).await?;

    let first = creating_res(&fixture, thread_id, 1, utc("2026-01-01 00:00:01"));
    let second = creating_res(&fixture, thread_id, 2, utc("2026-01-01 00:00:02"));
    insert_multiple_res(db, std::slice::from_ref(&first)).await?;
    insert_multiple_res(db, &[first.clone(), second]).await?;
    insert_multiple_res(db, &[first]).await?;

    assert_eq!(response_count(db, thread_id).await?, 2);
    let thread = thread::Entity::find_by_id(thread_id)
        .one(db)
        .await?
        .unwrap();
    assert_eq!(thread.response_count, 2);
    Ok(())
}

#[tokio::test]
async fn drops_only_rows_the_database_rejects() -> anyhow::Result<()> {
    let test_db = setup_database().await?;
    let db = &test_db.db;
    let fixture = insert_fixture(db).await?;
    let thread_id = insert_thread(db, &fixture, 1, timestamp("2026-01-01 00:00:00"), false).await?;

    let valid = creating_res(&fixture, thread_id, 1, utc("2026-01-01 00:00:01"));
    let deleted_thread = creating_res(&fixture, Uuid::now_v7(), 1, utc("2026-01-01 00:00:02"));
    let mut too_long = creating_res(&fixture, thread_id, 2, utc("2026-01-01 00:00:03"));
    too_long.body = "a".repeat(70_000);

    insert_multiple_res(
        db,
        &[valid.clone(), deleted_thread.clone(), too_long.clone()],
    )
    .await?;

    assert!(
        response::Entity::find_by_id(valid.id)
            .one(db)
            .await?
            .is_some()
    );
    assert!(
        response::Entity::find_by_id(deleted_thread.id)
            .one(db)
            .await?
            .is_none()
    );
    assert!(
        response::Entity::find_by_id(too_long.id)
            .one(db)
            .await?
            .is_none()
    );
    let thread = thread::Entity::find_by_id(thread_id)
        .one(db)
        .await?
        .unwrap();
    assert_eq!(thread.response_count, 1);
    Ok(())
}

#[tokio::test]
async fn thread_stats_keep_archived_inactive_and_never_rewind() -> anyhow::Result<()> {
    let test_db = setup_database().await?;
    let db = &test_db.db;
    let fixture = insert_fixture(db).await?;
    let newer = timestamp("2026-02-01 00:00:00");
    let archived_thread = insert_thread(db, &fixture, 1, newer, true).await?;
    let live_thread =
        insert_thread(db, &fixture, 2, timestamp("2026-01-01 00:00:00"), false).await?;

    insert_multiple_res(
        db,
        &[
            creating_res(&fixture, archived_thread, 1, utc("2026-01-15 00:00:00")),
            creating_res(&fixture, live_thread, 1, utc("2026-01-15 00:00:00")),
        ],
    )
    .await?;

    let archived = thread::Entity::find_by_id(archived_thread)
        .one(db)
        .await?
        .unwrap();
    assert!(!archived.active);
    assert_eq!(archived.response_count, 1);
    assert_eq!(archived.last_modified_at, newer);

    let live = thread::Entity::find_by_id(live_thread)
        .one(db)
        .await?
        .unwrap();
    assert!(live.active);
    assert_eq!(live.last_modified_at, timestamp("2026-01-15 00:00:00"));
    Ok(())
}

#[tokio::test]
async fn thread_over_limit_becomes_inactive() -> anyhow::Result<()> {
    let test_db = setup_database().await?;
    let db = &test_db.db;
    let fixture = insert_fixture(db).await?;
    let thread_id = insert_thread(db, &fixture, 1, timestamp("2026-01-01 00:00:00"), false).await?;

    let res_list = (1..=1001)
        .map(|order| creating_res(&fixture, thread_id, order, utc("2026-01-01 00:00:01")))
        .collect::<Vec<_>>();
    insert_multiple_res(db, &res_list[..1000]).await?;
    let thread = thread::Entity::find_by_id(thread_id)
        .one(db)
        .await?
        .unwrap();
    assert!(thread.active);

    insert_multiple_res(db, &res_list[1000..]).await?;
    let thread = thread::Entity::find_by_id(thread_id)
        .one(db)
        .await?
        .unwrap();
    assert_eq!(thread.response_count, 1001);
    assert!(!thread.active);
    Ok(())
}

#[tokio::test]
async fn created_at_is_truncated_not_rounded() -> anyhow::Result<()> {
    let test_db = setup_database().await?;
    let db = &test_db.db;
    let fixture = insert_fixture(db).await?;
    let thread_id = insert_thread(db, &fixture, 1, timestamp("2026-01-01 00:00:00"), false).await?;

    let res = creating_res(&fixture, thread_id, 1, utc("2026-01-01 00:00:00.9996"));
    insert_multiple_res(db, std::slice::from_ref(&res)).await?;

    let stored = response::Entity::find_by_id(res.id).one(db).await?.unwrap();
    assert_eq!(stored.created_at, timestamp("2026-01-01 00:00:00.999"));
    let thread = thread::Entity::find_by_id(thread_id)
        .one(db)
        .await?
        .unwrap();
    assert_eq!(
        thread.last_modified_at,
        timestamp("2026-01-01 00:00:00.999")
    );
    Ok(())
}
