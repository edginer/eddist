use super::*;
use eddist::repositories::bbs_repository::ThreadStatus;
use eddist_core::domain::tinker::Tinker;

async fn select_count(db: &TestDatabase) -> u64 {
    let (_, value): (String, String) = sqlx::query_as("SHOW SESSION STATUS LIKE 'Com_select'")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    value.parse().unwrap()
}

#[tokio::test]
async fn core_threads_preserve_status_projection_order_and_board_lookup() {
    let db = TestDatabase::new().await;
    let repo = db.bbs();
    let board = db.insert_board("core").await;
    let other_board = db.insert_board("other-core").await;
    let token = token_fixture("core", "192.0.2.1", "core-code");
    repo.create_authed_token(token.clone()).await.unwrap();
    assert!(
        repo.get_threads(board, ThreadStatus::Unarchived)
            .await
            .unwrap()
            .is_empty()
    );
    let mut inputs = Vec::new();
    for (index, active, archived, day) in [
        (0, true, false, 1),
        (1, false, false, 3),
        (2, false, true, 2),
        (3, true, true, 4),
    ] {
        let input = thread_fixture(board, token.id, 100 + index);
        repo.create_thread(input.clone()).await.unwrap();
        sqlx::query("UPDATE threads SET active = ?, archived = ?, no_pool = ?, archive_converted = TRUE, response_count = ?, last_modified_at = ?, sage_last_modified_at = ? WHERE id = ?")
            .bind(active).bind(archived).bind(index % 2 == 1).bind(900 + index as i32)
            .bind(time(day)).bind(time(day + 1)).bind(input.thread_id).execute(&db.pool).await.unwrap();
        inputs.push(input);
    }
    let other = thread_fixture(other_board, token.id, 100);
    repo.create_thread(other.clone()).await.unwrap();
    let before = select_count(&db).await;
    for (status, mut expected) in [
        (ThreadStatus::Active, vec![100, 103]),
        (ThreadStatus::Archived, vec![102, 103]),
        (ThreadStatus::Inactive, vec![101]),
        (ThreadStatus::Unarchived, vec![101, 100]),
    ] {
        let found = repo.get_threads(board, status).await.unwrap();
        let mut numbers: Vec<_> = found.iter().map(|row| row.thread_number).collect();
        if !matches!(status, ThreadStatus::Unarchived) {
            numbers.sort();
            expected.sort();
        }
        assert_eq!(numbers, expected);
        for row in found {
            let index = (row.thread_number - 100) as usize;
            assert_eq!(row.id, inputs[index].thread_id);
            assert_eq!(row.board_id, board);
            assert_eq!(row.title, inputs[index].title);
            assert_eq!(row.authed_token_id, token.id);
            assert_eq!(row.metadent, "vvv");
            assert_eq!(row.response_count, 900 + index as u32);
            assert_eq!(row.no_pool, index % 2 == 1);
            assert_eq!(row.active, index == 0 || index == 3);
            assert_eq!(row.archived, index >= 2);
            let day = [1, 3, 2, 4][index];
            assert_eq!(row.last_modified_at, time(day).and_utc());
            assert_eq!(row.sage_last_modified_at, time(day + 1).and_utc());
        }
    }
    let single = repo
        .get_thread_by_board_key_and_thread_number("other-core", 100)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(single.id, other.thread_id);
    assert!(
        repo.get_thread_by_board_key_and_thread_number("missing", 100)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        repo.get_thread_by_board_key_and_thread_number("core", 999)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        select_count(&db).await - before,
        7,
        "one SELECT per repository read"
    );
    assert!(
        repo.get_threads(Uuid::nil(), ThreadStatus::Active)
            .await
            .unwrap()
            .is_empty()
    );
    db.pool.close().await;
    for status in [
        ThreadStatus::Active,
        ThreadStatus::Archived,
        ThreadStatus::Inactive,
        ThreadStatus::Unarchived,
    ] {
        assert!(repo.get_threads(board, status).await.is_err());
    }
    assert!(
        repo.get_thread_by_board_key_and_thread_number("core", 100)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn core_responses_preserve_tied_order_flags_all_fields_and_dat_bytes() {
    let db = TestDatabase::new().await;
    let repo = db.bbs();
    let board = db.insert_board("core-res").await;
    let token = token_fixture("core-res", "192.0.2.1", "res-code");
    repo.create_authed_token(token.clone()).await.unwrap();
    let mut thread = thread_fixture(board, token.id, 100);
    thread.response_id = Uuid::from_u128(500);
    repo.create_thread(thread.clone()).await.unwrap();
    let mut expected = vec![(
        1,
        thread.response_id,
        ResView {
            author_name: thread.name.clone(),
            mail: thread.mail.clone(),
            body: thread.body.clone(),
            created_at: eddist_entity::db_time::truncate_to_millis(thread.created_at.naive_utc())
                .and_utc(),
            author_id: thread.author_ch5id.clone(),
            is_abone: false,
            is_abone_keep_id: false,
        },
    )];
    // Insert in a different order from (res_order, id), with deliberately tied order values.
    for (order, id, abone, keep_id, name, mail) in [
        (3, 503, true, false, "消す名前", ""),
        (2, 502, true, true, "IDを残す名前", "sage"),
        (2, 501, false, true, "", "email@example.com"),
        (4, 504, false, false, "通常の名前", "sage"),
    ] {
        let mut res = response_fixture(&thread, order);
        res.id = Uuid::from_u128(id);
        res.name = name.into();
        res.mail = mail.into();
        res.author_ch5id = format!("ID{id}");
        res.created_at = time(order as u32).and_utc();
        repo.create_response(res.clone()).await.unwrap();
        sqlx::query("UPDATE responses SET is_abone = ?, is_abone_keep_id = ? WHERE id = ?")
            .bind(abone)
            .bind(keep_id)
            .bind(res.id)
            .execute(&db.pool)
            .await
            .unwrap();
        expected.push((
            order,
            res.id,
            ResView {
                author_name: res.name,
                mail: res.mail,
                body: res.body,
                created_at: res.created_at,
                author_id: res.author_ch5id,
                is_abone: abone,
                is_abone_keep_id: keep_id,
            },
        ));
    }
    expected.sort_by_key(|(order, id, _)| (*order, *id));
    let before = select_count(&db).await;
    let found = repo.get_responses(thread.thread_id).await.unwrap();
    assert_eq!(select_count(&db).await - before, 1);
    assert_eq!(found.len(), expected.len());
    for (index, (actual, (_, _, expected))) in found.iter().zip(&expected).enumerate() {
        assert_eq!(format!("{actual:?}"), format!("{expected:?}"));
        let title = (index == 0).then_some(thread.title.as_str());
        assert_eq!(
            actual.get_sjis_bytes("名無さん", title).get_inner(),
            expected.get_sjis_bytes("名無さん", title).get_inner()
        );
    }
    assert!(repo.get_responses(Uuid::nil()).await.unwrap().is_empty());
    db.pool.close().await;
    assert!(repo.get_responses(thread.thread_id).await.is_err());
}

#[tokio::test]
async fn core_metadent_preserves_join_current_token_and_scalar_subquery_errors() {
    let db = TestDatabase::new().await;
    let repo = db.bbs();
    let board = db.insert_board("core-meta").await;
    let other_board = db.insert_board("other-meta").await;
    let mut tokens = Vec::new();
    for (name, ip) in [
        ("meta-first", "192.0.2.1"),
        ("meta-current", "2001:db8::1234"),
    ] {
        let token = token_fixture(name, ip, name);
        repo.create_authed_token(token.clone()).await.unwrap();
        tokens.push(token);
    }
    let idp = db.insert_idp("meta-idp", true).await;
    let user = user_fixture(idp, "metadent user");
    db.register_user(user.clone(), tokens[1].id).await.unwrap();
    sqlx::query("UPDATE authed_tokens SET authed_ua = 'current UA', validity = TRUE, authed_at = ?, last_wrote_at = ?, require_user_registration = TRUE, require_reauth = TRUE WHERE id = ?")
        .bind(time(3)).bind(time(4)).bind(tokens[1].id).execute(&db.pool).await.unwrap();
    let mut inputs = Vec::new();
    for (index, active, archived, board_id) in [
        (0, true, false, board),
        (1, false, false, board),
        (2, false, true, board),
        (3, true, false, other_board),
    ] {
        let mut input = thread_fixture(board_id, tokens[0].id, 100 + index);
        input.client_info.user_agent = format!("original client {index}");
        input.client_info.tinker = Some(Box::new(Tinker::new(
            "original token".into(),
            time(1).and_utc(),
        )));
        repo.create_thread(input.clone()).await.unwrap();
        sqlx::query("UPDATE threads SET active = ?, archived = ?, no_pool = TRUE, archive_converted = TRUE, authed_token_id = ?, sage_last_modified_at = ? WHERE id = ?")
            .bind(active).bind(archived).bind(tokens[1].id).bind(time(index as u32 + 1)).bind(input.thread_id).execute(&db.pool).await.unwrap();
        inputs.push(input);
    }
    let before = select_count(&db).await;
    let found = repo.get_threads_with_metadent(board).await.unwrap();
    assert_eq!(
        select_count(&db).await - before,
        1,
        "metadent must remain a single SELECT"
    );
    assert_eq!(found.len(), 2);
    assert_eq!(
        found
            .iter()
            .map(|(t, _, _)| t.thread_number)
            .collect::<Vec<_>>(),
        [101, 100]
    );
    let mut expected_token = repo
        .get_authed_token_by_id(tokens[1].id)
        .await
        .unwrap()
        .unwrap();
    expected_token.asn_num = 0; // Existing metadent DTO intentionally does not project token ASN.
    for (thread, client, token) in &found {
        let input = &inputs[(thread.thread_number - 100) as usize];
        let expected_thread = repo
            .get_thread_by_board_key_and_thread_number("core-meta", input.unix_time)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(format!("{thread:?}"), format!("{expected_thread:?}"));
        assert_eq!(
            serde_json::to_value(client).unwrap(),
            serde_json::to_value(&input.client_info).unwrap()
        );
        assert_eq!(format!("{token:?}"), format!("{expected_token:?}"));
    }
    assert!(
        !found[0].0.active,
        "inactive, unarchived threads must remain in the list"
    );
    assert!(
        repo.get_threads_with_metadent(Uuid::nil())
            .await
            .unwrap()
            .is_empty()
    );

    let first = &inputs[0];
    sqlx::query("UPDATE responses SET res_order = 2 WHERE id = ?")
        .bind(first.response_id)
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(
        repo.get_threads_with_metadent(board).await.is_err(),
        "missing first response is an error, not a missing thread"
    );
    sqlx::query("UPDATE responses SET res_order = 1 WHERE id = ?")
        .bind(first.response_id)
        .execute(&db.pool)
        .await
        .unwrap();
    for json in [serde_json::Value::Null, serde_json::json!({"invalid":true})] {
        sqlx::query("UPDATE responses SET client_info = ? WHERE id = ?")
            .bind(json)
            .bind(first.response_id)
            .execute(&db.pool)
            .await
            .unwrap();
        assert!(
            repo.get_threads_with_metadent(board).await.is_err(),
            "malformed ClientInfo must not become a default value"
        );
    }
    sqlx::query("UPDATE responses SET client_info = ? WHERE id = ?")
        .bind(serde_json::to_value(&first.client_info).unwrap())
        .bind(first.response_id)
        .execute(&db.pool)
        .await
        .unwrap();
    assert_eq!(
        repo.get_threads_with_metadent(board).await.unwrap().len(),
        2
    );
    repo.create_response(response_fixture(first, 1))
        .await
        .unwrap();
    assert!(
        repo.get_threads_with_metadent(board).await.is_err(),
        "multiple first responses must remain a scalar-subquery error"
    );
    db.pool.close().await;
    assert!(repo.get_threads_with_metadent(board).await.is_err());
}
