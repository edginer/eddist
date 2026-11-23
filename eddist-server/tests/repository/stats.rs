use super::*;

#[tokio::test]
async fn stats_preserve_jst_calendar_date_range_and_order() {
    let db = TestDatabase::new().await;
    let repo = StatsRepositoryImpl::new(db.orm());
    // Record the SQLx calendar contract independently of the host's timezone.
    for (utc, expected) in [
        ("2020-01-01T14:59:59Z", "2020-01-01"),
        ("2020-01-01T15:00:00Z", "2020-01-02"),
        ("2020-12-31T15:00:00Z", "2021-01-01"),
        ("2020-02-28T15:00:00Z", "2020-02-29"),
    ] {
        let timestamp = chrono::DateTime::parse_from_rfc3339(utc)
            .unwrap()
            .timestamp();
        sqlx::query("SET timestamp = ?")
            .bind(timestamp)
            .execute(&db.pool)
            .await
            .unwrap();
        let date: NaiveDate =
            sqlx::query_scalar("SELECT DATE(CONVERT_TZ(NOW(), '+00:00', '+09:00'))")
                .fetch_one(&db.pool)
                .await
                .unwrap();
        assert_eq!(date.to_string(), expected);
    }
    sqlx::query("SET timestamp = 0")
        .execute(&db.pool)
        .await
        .unwrap();
    let today: NaiveDate = sqlx::query_scalar("SELECT DATE(CONVERT_TZ(NOW(), '+00:00', '+09:00'))")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert!(repo.get_today_stats_per_board().await.unwrap().is_empty());
    assert!(repo.get_daily_stats_per_board(30).await.unwrap().is_empty());
    for (key, days_ago, responses, threads) in [
        ("a", 3, 30_i64, 3_i64),
        ("b", 2, 21, 3),
        ("a", 2, 20, 2),
        ("a", 1, 10, 1),
        ("a", 0, 100, 10),
        ("future", -1, 200, 20),
    ] {
        sqlx::query("INSERT INTO daily_stats (date, board_key, total_responses, new_threads) VALUES (?, ?, ?, ?)")
            .bind(today - chrono::Duration::days(days_ago)).bind(key).bind(responses).bind(threads)
            .execute(&db.pool).await.unwrap();
    }
    let rows = repo.get_today_stats_per_board().await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].date, today);
    assert_eq!(rows[0].board_key, "a");
    assert_eq!(rows[0].total_responses, 100);
    assert_eq!(rows[0].new_threads, 10);
    let rows = repo.get_daily_stats_per_board(2).await.unwrap();
    let values: Vec<_> = rows
        .into_iter()
        .map(|r| (r.board_key, r.date, r.total_responses, r.new_threads))
        .collect();
    assert_eq!(
        values,
        [
            ("a".into(), today - chrono::Duration::days(1), 10, 1),
            ("a".into(), today - chrono::Duration::days(2), 20, 2),
            ("b".into(), today - chrono::Duration::days(2), 21, 3),
        ]
    );
    assert!(repo.get_daily_stats_per_board(0).await.unwrap().is_empty());
    assert!(
        repo.get_daily_stats_per_board(u32::MAX)
            .await
            .unwrap()
            .is_empty()
    );
    db.pool.close().await;
    assert!(repo.get_today_stats_per_board().await.is_err());
    assert!(repo.get_daily_stats_per_board(2).await.is_err());
    assert!(repo.flush_board_stats(&[("a".into(), 1, 1)]).await.is_err());
}

#[tokio::test]
async fn stats_flush_adds_per_board_and_rolls_back_the_entire_snapshot() {
    let db = TestDatabase::with_max_connections(4).await;
    let mut orm = db.orm();
    let statements = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    orm.set_metric_callback({
        let statements = statements.clone();
        move |info| statements.lock().unwrap().push(info.statement.sql.clone())
    });
    let repo = StatsRepositoryImpl::new(orm);
    repo.flush_board_stats(&[("a".into(), 10, 2), ("b".into(), 20, 3)])
        .await
        .unwrap();
    // No model reload SELECT is needed: one DML statement per snapshot entry.
    {
        let sql = statements.lock().unwrap();
        assert_eq!(sql.len(), 2);
        assert!(sql.iter().all(|sql| sql.starts_with("INSERT INTO")));
    }
    sqlx::query("UPDATE daily_stats SET created_at = ? WHERE board_key = 'a'")
        .bind(time(1))
        .execute(&db.pool)
        .await
        .unwrap();
    repo.flush_board_stats(&[("a".into(), 3, 1), ("a".into(), -1, 0), ("c".into(), 5, 0)])
        .await
        .unwrap();
    repo.flush_board_stats(&[]).await.unwrap();
    repo.flush_board_stats(&[("a".into(), 0, 0)]).await.unwrap();
    let left = [("a".into(), 2, 1)];
    let right = [("a".into(), 3, 1)];
    let (left, right) = tokio::join!(
        repo.flush_board_stats(&left),
        repo.flush_board_stats(&right)
    );
    left.unwrap();
    right.unwrap();
    let mut before = repo.get_today_stats_per_board().await.unwrap();
    before.sort_by(|a, b| a.board_key.cmp(&b.board_key));
    let before: Vec<_> = before
        .into_iter()
        .map(|r| (r.board_key, r.total_responses, r.new_threads))
        .collect();
    assert_eq!(
        before,
        [("a".into(), 17, 5), ("b".into(), 20, 3), ("c".into(), 5, 0)]
    );

    // The second insert fails after the first update; neither may be committed.
    assert!(
        repo.flush_board_stats(&[("a".into(), 100, 10), ("x".repeat(256), 1, 1)])
            .await
            .is_err()
    );
    let mut after = repo.get_today_stats_per_board().await.unwrap();
    after.sort_by(|a, b| a.board_key.cmp(&b.board_key));
    let after: Vec<_> = after
        .into_iter()
        .map(|r| (r.board_key, r.total_responses, r.new_threads))
        .collect();
    assert_eq!(after, before);
    let created_at: NaiveDateTime =
        sqlx::query_scalar("SELECT created_at FROM daily_stats WHERE board_key = 'a'")
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(created_at, time(1));
}
