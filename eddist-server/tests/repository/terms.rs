use super::*;

#[tokio::test]
async fn terms_select_latest_update_and_preserve_empty_result() {
    let db = TestDatabase::new().await;
    sqlx::query("DELETE FROM terms")
        .execute(&db.pool)
        .await
        .unwrap();
    let repo = TermsRepositoryImpl::new(db.orm());
    assert!(repo.get_terms().await.unwrap().is_none());

    let mut latest_id = Uuid::nil();
    for (day, updated_by) in [(3, Some("editor@example.com")), (1, None), (2, None)] {
        let id = Uuid::now_v7();
        if day == 3 {
            latest_id = id;
        }
        sqlx::query("INSERT INTO terms (id, content, created_at, updated_at, updated_by) VALUES (UUID_TO_BIN(?), ?, ?, ?, ?)")
            .bind(id.to_string()).bind(format!("規約 {day}")).bind(time(1)).bind(time(day)).bind(updated_by)
            .execute(&db.pool).await.unwrap();
    }
    let terms = repo.get_terms().await.unwrap().unwrap();
    assert_eq!(terms.id, latest_id);
    assert_eq!(terms.content, "規約 3");
    assert_eq!(terms.created_at, time(1));
    assert_eq!(terms.updated_at, time(3));
    assert_eq!(terms.updated_by.as_deref(), Some("editor@example.com"));
    sqlx::query("UPDATE terms SET updated_by = NULL WHERE id = UUID_TO_BIN(?)")
        .bind(latest_id.to_string())
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(
        repo.get_terms()
            .await
            .unwrap()
            .unwrap()
            .updated_by
            .is_none()
    );
    db.pool.close().await;
    assert!(repo.get_terms().await.is_err());
}
