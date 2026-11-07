//! Exercise the production factory without the fixture pool's session settings.
use super::*;
use std::{collections::HashSet, time::Instant};

#[tokio::test]
async fn native_pool_preserves_capacity_timeout_and_shared_ownership() {
    let db = TestDatabase::new().await;
    let native = eddist::database::connect_database(&db.url().await)
        .await
        .unwrap();
    let pool = native.get_mysql_connection_pool();
    assert_eq!(pool.options().get_max_connections(), 8);
    assert_eq!(pool.options().get_acquire_timeout(), Duration::from_secs(5));

    let mut leases = Vec::new();
    let mut connection_ids = HashSet::new();
    for _ in 0..8 {
        let mut lease = pool.acquire().await.unwrap();
        let (id, sql_mode): (u64, String) =
            sqlx::query_as("SELECT CONNECTION_ID(), @@SESSION.sql_mode")
                .fetch_one(&mut *lease)
                .await
                .unwrap();
        assert!(!sql_mode.contains("TIME_TRUNCATE_FRACTIONAL"));
        connection_ids.insert(id);
        leases.push(lease);
    }
    assert_eq!(connection_ids.len(), 8);
    let started = Instant::now();
    assert!(matches!(
        pool.acquire().await,
        Err(sqlx::Error::PoolTimedOut)
    ));
    assert!(started.elapsed() >= Duration::from_millis(4500));
    drop(leases);

    let board_id = db.insert_board("native").await;
    let repo = BbsRepositoryImpl::new(native.clone());
    assert_eq!(
        repo.get_board("native").await.unwrap().unwrap().id,
        board_id
    );
    native.close_by_ref().await.unwrap();
    assert!(repo.get_board("native").await.is_err());
    // Closing a clone affects the app's pool, not the separate test fixture pool.
    assert_eq!(
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&db.pool)
            .await
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn native_writes_keep_milliseconds_and_explicit_read_committed() {
    let db = TestDatabase::new().await;
    let native = eddist::database::connect_database(&db.url().await)
        .await
        .unwrap();
    let pool = native.get_mysql_connection_pool();
    let isolation: String = sqlx::query_scalar("SELECT @@SESSION.transaction_isolation")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(isolation, "REPEATABLE-READ");
    let repo = BbsRepositoryImpl::new(native.clone());
    let board_id = db.insert_board("native-write").await;
    let idp_id = db.insert_idp("native", true).await;
    let first = token_fixture("first", "192.0.2.1", "123456");
    let mut second = token_fixture("second", "192.0.2.2", "654321");
    second.author_id_seed = vec![99; 64];
    for token in [&first, &second] {
        repo.create_authed_token(token.clone()).await.unwrap();
    }
    repo.activate_authed_status(&first.token, "UA", first.created_at, None)
        .await
        .unwrap();
    repo.update_authed_token_last_wrote(first.id, first.created_at)
        .await
        .unwrap();

    // General logging is confined to this disposable container and records the
    // isolation command sent before each of the five transaction boundaries.
    sqlx::query("SET GLOBAL log_output = 'TABLE'")
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::query("SET GLOBAL general_log = 'ON'")
        .execute(&db.pool)
        .await
        .unwrap();
    let thread = thread_fixture(board_id, first.id, 1_600_000_000);
    repo.create_thread(thread.clone()).await.unwrap();
    let response = response_fixture(&thread, 2);
    repo.create_response(response.clone()).await.unwrap();
    let user = user_fixture(idp_id, "native-user");
    let user_id = user.user_id;
    let users = UserRepositoryImpl::new(native.clone());
    users
        .register_user_and_bind_token(user, first.id)
        .await
        .unwrap();
    users
        .bind_user_authed_token(user_id, second.id)
        .await
        .unwrap();
    StatsRepositoryImpl::new(native.clone())
        .flush_board_stats(&[("native-write".into(), 2, 1)])
        .await
        .unwrap();
    sqlx::query("SET GLOBAL general_log = 'OFF'")
        .execute(&db.pool)
        .await
        .unwrap();
    let commands: Vec<Vec<u8>> =
        sqlx::query_scalar("SELECT argument FROM mysql.general_log WHERE command_type = 'Execute'")
            .fetch_all(&db.pool)
            .await
            .unwrap();
    let isolation_commands: Vec<_> = commands
        .iter()
        .map(|command| String::from_utf8_lossy(command).to_uppercase())
        .filter(|command| command.starts_with("SET TRANSACTION ISOLATION LEVEL"))
        .collect();
    assert_eq!(
        isolation_commands,
        vec!["SET TRANSACTION ISOLATION LEVEL READ COMMITTED"; 5]
    );

    let token = repo.get_authed_token(&first.token).await.unwrap().unwrap();
    let expected = first.created_at.with_nanosecond(999_000_000).unwrap();
    assert_eq!(token.created_at, expected);
    assert_eq!(token.authed_at, Some(expected));
    assert_eq!(token.last_wrote_at, Some(expected));
    let stored = repo
        .get_thread_by_board_key_and_thread_number("native-write", thread.unix_time)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        stored.last_modified_at,
        response.created_at.with_nanosecond(999_000_000).unwrap()
    );
    let responses = repo.get_responses(thread.thread_id).await.unwrap();
    assert_eq!(
        responses[0].created_at,
        thread.created_at.with_nanosecond(999_000_000).unwrap()
    );
    assert_eq!(
        responses[1].created_at,
        response.created_at.with_nanosecond(999_000_000).unwrap()
    );
    let second = repo.get_authed_token(&second.token).await.unwrap().unwrap();
    assert_eq!(second.author_id_seed, first.author_id_seed);
    assert_eq!(second.registered_user_id, Some(user_id));
    let created_at: NaiveDateTime = sqlx::query_scalar("SELECT created_at FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(created_at.nanosecond(), 0);
}
