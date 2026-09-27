use chrono::{NaiveDateTime, TimeDelta, Utc};
use eddist_entity::{authed_token, board, thread, user, user_authed_token};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, Database, DatabaseConnection, EntityTrait};
use testcontainers::{ContainerAsync, ImageExt, core::IntoContainerPort, runners::AsyncRunner};
use testcontainers_modules::mysql::Mysql;
use uuid::Uuid;

use crate::repository::{Repository, StaleAuthedTokenKind};

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

async fn insert_token(
    db: &DatabaseConnection,
    created_at: NaiveDateTime,
    authed_at: Option<NaiveDateTime>,
    validity: bool,
    last_wrote_at: Option<NaiveDateTime>,
    registered_user_id: Option<Uuid>,
) -> anyhow::Result<Uuid> {
    let id = Uuid::now_v7();
    authed_token::ActiveModel {
        id: Set(id),
        token: Set(id.simple().to_string()),
        origin_ip: Set("192.0.2.1".to_string()),
        reduced_origin_ip: Set("192.0.2.0".to_string()),
        writing_ua: Set("test-ua".to_string()),
        authed_ua: Set(authed_at.map(|_| "test-ua".to_string())),
        auth_code: Set("000000".to_string()),
        created_at: Set(created_at),
        authed_at: Set(authed_at),
        validity: Set(validity),
        last_wrote_at: Set(last_wrote_at),
        asn_num: Set(0),
        additional_info: Set(None),
        require_user_registration: Set(false),
        registered_user_id: Set(registered_user_id),
        require_reauth: Set(false),
        author_id_seed: Set(vec![0; 64]),
    }
    .insert(db)
    .await?;
    Ok(id)
}

#[tokio::test]
async fn delete_stale_authed_tokens_keeps_referenced_and_recent_tokens() -> anyhow::Result<()> {
    let test_database = setup_database().await?;
    let db = &test_database.db;
    let now = Utc::now().naive_utc();
    let days_ago = |days| now - TimeDelta::days(days);

    let pending_old = insert_token(db, days_ago(8), None, false, None, None).await?;
    let pending_recent = insert_token(db, days_ago(1), None, false, None, None).await?;
    let revoked_old = insert_token(
        db,
        days_ago(800),
        Some(days_ago(800)),
        false,
        Some(days_ago(800)),
        None,
    )
    .await?;

    let idle_old = insert_token(
        db,
        days_ago(800),
        Some(days_ago(800)),
        true,
        Some(days_ago(401)),
        None,
    )
    .await?;
    let idle_never_wrote =
        insert_token(db, days_ago(401), Some(days_ago(401)), true, None, None).await?;
    let active = insert_token(
        db,
        days_ago(800),
        Some(days_ago(800)),
        true,
        Some(days_ago(10)),
        None,
    )
    .await?;

    let user_id = Uuid::now_v7();
    user::ActiveModel {
        id: Set(user_id),
        user_name: Set("user".to_string()),
        enabled: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await?;
    let idle_registered = insert_token(
        db,
        days_ago(800),
        Some(days_ago(800)),
        true,
        Some(days_ago(401)),
        Some(user_id),
    )
    .await?;
    let idle_user_linked = insert_token(
        db,
        days_ago(800),
        Some(days_ago(800)),
        true,
        Some(days_ago(401)),
        None,
    )
    .await?;
    user_authed_token::ActiveModel {
        id: Set(Uuid::now_v7()),
        user_id: Set(user_id),
        authed_token_id: Set(idle_user_linked),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await?;

    let board_id = Uuid::now_v7();
    board::ActiveModel {
        id: Set(board_id),
        name: Set("board".to_string()),
        board_key: Set("board".to_string()),
        default_name: Set("名無し".to_string()),
    }
    .insert(db)
    .await?;
    let idle_thread_owner = insert_token(
        db,
        days_ago(800),
        Some(days_ago(800)),
        true,
        Some(days_ago(401)),
        None,
    )
    .await?;
    thread::ActiveModel {
        id: Set(Uuid::now_v7()),
        board_id: Set(board_id),
        thread_number: Set(1),
        last_modified_at: Set(days_ago(401)),
        sage_last_modified_at: Set(days_ago(401)),
        title: Set("title".to_string()),
        authed_token_id: Set(idle_thread_owner),
        metadent: Set(String::new()),
        response_count: Set(1),
        no_pool: Set(false),
        active: Set(false),
        archived: Set(true),
        archive_converted: Set(true),
    }
    .insert(db)
    .await?;

    let repo = Repository::new(db.clone());
    let mut pending = repo
        .delete_stale_authed_tokens(StaleAuthedTokenKind::Pending, days_ago(7), 1)
        .await?;
    pending.sort();
    assert_eq!(pending, vec![pending_old]);

    let mut idle = repo
        .delete_stale_authed_tokens(StaleAuthedTokenKind::Idle, days_ago(400), 1)
        .await?;
    idle.sort();
    let mut expected_idle = vec![idle_old, idle_never_wrote];
    expected_idle.sort();
    assert_eq!(idle, expected_idle);

    let mut remaining = authed_token::Entity::find()
        .all(db)
        .await?
        .into_iter()
        .map(|t| t.id)
        .collect::<Vec<_>>();
    remaining.sort();
    let mut expected_remaining = vec![
        pending_recent,
        revoked_old,
        active,
        idle_registered,
        idle_user_linked,
        idle_thread_owner,
    ];
    expected_remaining.sort();
    assert_eq!(remaining, expected_remaining);

    Ok(())
}
