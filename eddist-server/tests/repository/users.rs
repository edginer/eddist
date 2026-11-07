use super::*;

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
