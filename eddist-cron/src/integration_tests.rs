use chrono::{DateTime, Duration, TimeZone, Utc};
use eddist_entity::{
    archived_response, archived_thread, authed_token, board, board_info, response, thread,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectOptions, Database, DatabaseConnection,
    EntityTrait, PaginatorTrait, QueryFilter,
};
use testcontainers::{
    ContainerAsync, Image, ImageExt, core::IntoContainerPort, runners::AsyncRunner,
};
use testcontainers_modules::{mysql::Mysql, postgres::Postgres};
use uuid::Uuid;

use crate::repository::Repository;

const BOARD_KEY: &str = "cron-it";

struct TestDatabase<I: Image> {
    _container: ContainerAsync<I>,
    db: DatabaseConnection,
}

async fn connect(database_url: String) -> anyhow::Result<DatabaseConnection> {
    let mut options = ConnectOptions::new(database_url);
    options.sqlx_logging(false);
    Ok(Database::connect(options).await?)
}

async fn setup_mysql() -> anyhow::Result<TestDatabase<Mysql>> {
    let container = Mysql::default().with_tag("8.0").start().await?;
    let port = container.get_host_port_ipv4(3306.tcp()).await?;
    let database_url = format!("mysql://root@127.0.0.1:{port}/test");

    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .connect(&database_url)
        .await?;
    sqlx::migrate!("../migrations").run(&pool).await?;
    pool.close().await;

    Ok(TestDatabase {
        _container: container,
        db: connect(database_url).await?,
    })
}

async fn setup_postgres() -> anyhow::Result<TestDatabase<Postgres>> {
    let container = Postgres::default().with_tag("18").start().await?;
    let port = container.get_host_port_ipv4(5432.tcp()).await?;
    let database_url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");

    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect(&database_url)
        .await?;
    sqlx::migrate!("../migrations/pg").run(&pool).await?;
    pool.close().await;

    Ok(TestDatabase {
        _container: container,
        db: connect(database_url).await?,
    })
}

fn base_time() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 1, 12, 0, 0).unwrap() + Duration::milliseconds(123)
}

async fn seed_board(db: &DatabaseConnection) -> anyhow::Result<Uuid> {
    let board_id = Uuid::now_v7();
    board::ActiveModel {
        id: Set(board_id),
        name: Set("cron integration".to_string()),
        board_key: Set(BOARD_KEY.to_string()),
        default_name: Set("名無しさん".to_string()),
    }
    .insert(db)
    .await?;
    board_info::ActiveModel {
        id: Set(board_id),
        local_rules: Set("rules".to_string()),
        base_thread_creation_span_sec: Set(60),
        base_response_creation_span_sec: Set(5),
        max_thread_name_byte_length: Set(96),
        max_author_name_byte_length: Set(64),
        max_email_byte_length: Set(64),
        max_response_body_byte_length: Set(4096),
        max_response_body_lines: Set(32),
        threads_archive_cron: Set(Some("0 0 * * * *".to_string())),
        threads_archive_trigger_thread_count: Set(Some(1)),
        read_only: Set(false),
        created_at: Set(base_time()),
        updated_at: Set(base_time()),
        force_metadent_type: Set(None),
        enable_1001_message: Set(true),
        custom_1001_message: Set(None),
    }
    .insert(db)
    .await?;
    Ok(board_id)
}

async fn seed_token(db: &DatabaseConnection) -> anyhow::Result<Uuid> {
    let id = Uuid::now_v7();
    authed_token::ActiveModel {
        id: Set(id),
        token: Set(format!("cron-token-{id}")),
        origin_ip: Set("127.0.0.1".to_string()),
        reduced_origin_ip: Set("127.0.0.1".to_string()),
        writing_ua: Set("cron-integration".to_string()),
        auth_code: Set("itest".to_string()),
        created_at: Set(base_time()),
        validity: Set(true),
        asn_num: Set(64512),
        require_user_registration: Set(false),
        require_reauth: Set(false),
        author_id_seed: Set(vec![0; 64]),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(id)
}

async fn seed_thread(
    db: &DatabaseConnection,
    board_id: Uuid,
    token_id: Uuid,
    thread_number: i64,
    last_modified_at: DateTime<Utc>,
) -> anyhow::Result<Uuid> {
    let id = Uuid::now_v7();
    thread::ActiveModel {
        id: Set(id),
        board_id: Set(board_id),
        thread_number: Set(thread_number),
        last_modified_at: Set(last_modified_at),
        sage_last_modified_at: Set(last_modified_at),
        title: Set(format!("thread-{thread_number}")),
        authed_token_id: Set(token_id),
        metadent: Set(String::new()),
        response_count: Set(2),
        no_pool: Set(false),
        active: Set(true),
        archived: Set(false),
        archive_converted: Set(false),
    }
    .insert(db)
    .await?;
    Ok(id)
}

async fn seed_response(
    db: &DatabaseConnection,
    board_id: Uuid,
    thread_id: Uuid,
    token_id: Uuid,
    res_order: i32,
    created_at: DateTime<Utc>,
) -> anyhow::Result<()> {
    response::ActiveModel {
        id: Set(Uuid::now_v7()),
        author_name: Set(format!("author-{res_order}")),
        mail: Set("sage".to_string()),
        body: Set(format!("body-{res_order}")),
        created_at: Set(created_at),
        author_id: Set("author".to_string()),
        ip_addr: Set("127.0.0.1".to_string()),
        authed_token_id: Set(token_id),
        board_id: Set(board_id),
        thread_id: Set(thread_id),
        is_abone: Set(false),
        res_order: Set(res_order),
        client_info: Set(serde_json::json!({
            "user_agent": "cron-integration",
            "asn_num": 64512,
            "ip_addr": "127.0.0.1",
            "tinker": null
        })),
        is_abone_keep_id: Set(res_order == 2),
    }
    .insert(db)
    .await?;
    Ok(())
}

async fn exercise_repository(db: &DatabaseConnection) -> anyhow::Result<()> {
    let repo = Repository::new(db.clone());
    let board_id = seed_board(db).await?;
    let token_id = seed_token(db).await?;
    let old_modified = base_time();
    let old_thread = seed_thread(db, board_id, token_id, 100, old_modified).await?;
    let new_thread = seed_thread(
        db,
        board_id,
        token_id,
        200,
        old_modified + Duration::seconds(1),
    )
    .await?;
    let first_created = base_time() + Duration::milliseconds(1);
    let second_created = base_time() + Duration::milliseconds(2);
    seed_response(db, board_id, old_thread, token_id, 1, first_created).await?;
    seed_response(db, board_id, old_thread, token_id, 2, second_created).await?;

    let boards = repo.get_all_boards_info().await?;
    let info = boards
        .iter()
        .find(|board| board.board_key == BOARD_KEY)
        .expect("seeded board is listed");
    assert_eq!(info.board_id, board_id);
    assert_eq!(info.threads_archive_trigger_thread_count, Some(1));
    assert!(info.enable_1001_message);

    repo.update_threads_to_inactive(BOARD_KEY, 1).await?;
    assert_eq!(
        repo.get_inactive_thread_numbers_for_board(BOARD_KEY)
            .await?,
        vec![100]
    );
    let untouched = thread::Entity::find_by_id(new_thread)
        .one(db)
        .await?
        .unwrap();
    assert!(untouched.active && !untouched.archived);

    let pending = repo
        .get_threads_with_archive_converted(BOARD_KEY, false)
        .await?;
    assert_eq!(
        pending,
        vec![("thread-100".to_string(), 100, old_thread, old_modified)]
    );

    let responses = repo.get_thread_responses(old_thread).await?;
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0].0.created_at, first_created);
    assert_eq!(responses[1].0.body, "body-2");
    assert!(responses[1].0.is_abone_keep_id);
    assert_eq!(responses[0].1.user_agent, "cron-integration");
    assert_eq!(responses[0].2, token_id);

    repo.update_archive_converted(old_thread).await?;
    assert!(
        repo.get_threads_with_archive_converted(BOARD_KEY, false)
            .await?
            .is_empty()
    );
    assert_eq!(
        repo.get_threads_with_archive_converted(BOARD_KEY, true)
            .await?
            .len(),
        1
    );

    repo.archive_thread_and_responses(old_thread).await?;
    assert!(
        thread::Entity::find_by_id(old_thread)
            .one(db)
            .await?
            .is_none()
    );
    assert_eq!(
        response::Entity::find()
            .filter(response::Column::ThreadId.eq(old_thread))
            .count(db)
            .await?,
        0
    );
    let archived = archived_thread::Entity::find()
        .filter(archived_thread::Column::Id.eq(old_thread))
        .one(db)
        .await?
        .unwrap();
    assert_eq!(archived.last_modified_at, old_modified);
    assert_eq!(
        archived_response::Entity::find()
            .filter(archived_response::Column::ThreadId.eq(old_thread))
            .count(db)
            .await?,
        2
    );

    assert_eq!(
        repo.get_archived_threads(BOARD_KEY, 1, 150).await?,
        vec![("thread-100".to_string(), 100, old_thread)]
    );
    let archived_responses = repo.get_archived_thread_responses(old_thread).await?;
    assert_eq!(archived_responses.len(), 2);
    assert_eq!(archived_responses[1].0.created_at, second_created);
    assert!(
        archived_responses
            .iter()
            .all(|(res, _, _)| !res.is_abone_keep_id)
    );

    repo.archive_thread_and_responses(old_thread).await?;

    Ok(())
}

#[tokio::test]
async fn repository_works_on_mysql() -> anyhow::Result<()> {
    let database = setup_mysql().await?;
    exercise_repository(&database.db).await
}

#[tokio::test]
async fn repository_works_on_postgres() -> anyhow::Result<()> {
    let database = setup_postgres().await?;
    exercise_repository(&database.db).await
}
