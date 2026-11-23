use super::*;

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
