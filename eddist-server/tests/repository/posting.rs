use super::*;

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
