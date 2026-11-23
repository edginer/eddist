use super::*;

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
