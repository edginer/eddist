use super::*;

#[tokio::test]
async fn notices_preserve_visibility_pagination_and_optional_fields() {
    let db = TestDatabase::new().await;
    let repo = NoticeRepositoryImpl::new(db.orm());
    assert!(repo.get_notices_paginated(0, 2).await.unwrap().is_empty());
    assert_eq!(repo.count_notices().await.unwrap(), 0);
    assert!(repo.get_notice_by_slug("missing").await.unwrap().is_none());

    let mut ids = Vec::new();
    for (slug, published_at, hidden, email) in [
        ("old", time(1), false, None),
        ("new", time(3), false, Some("author@example.com")),
        ("middle", time(2), false, None),
        ("hidden", time(4), true, None),
        (
            "future",
            NaiveDate::from_ymd_opt(2099, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            false,
            None,
        ),
    ] {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO notices (id, slug, title, content, created_at, updated_at, published_at, author_email, hide_from_list) VALUES (UUID_TO_BIN(?), ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(id.to_string()).bind(slug).bind(format!("お知らせ {slug}"))
            .bind("本文\n日本語").bind(time(1)).bind(time(2)).bind(published_at)
            .bind(email).bind(hidden).execute(&db.pool).await.unwrap();
        ids.push(id);
    }

    assert_eq!(repo.count_notices().await.unwrap(), 3);
    let page = repo.get_notices_paginated(0, 2).await.unwrap();
    assert_eq!(
        page.iter().map(|n| n.slug.as_str()).collect::<Vec<_>>(),
        ["new", "middle"]
    );
    assert_eq!(page[0].id, ids[1]);
    assert_eq!(page[0].title, "お知らせ new");
    assert_eq!(page[0].published_at, time(3));
    let page = repo.get_notices_paginated(1, 2).await.unwrap();
    assert_eq!(page.len(), 1);
    assert_eq!(page[0].slug, "old");
    assert!(repo.get_notices_paginated(2, 2).await.unwrap().is_empty());
    assert!(repo.get_notices_paginated(0, 0).await.unwrap().is_empty());
    assert!(repo.get_notice_by_slug("future").await.unwrap().is_none());

    let hidden = repo.get_notice_by_slug("hidden").await.unwrap().unwrap();
    assert_eq!(hidden.id, ids[3]);
    assert!(hidden.hide_from_list);
    assert_eq!(hidden.author_email, None);
    assert_eq!(hidden.content, "本文\n日本語");
    assert_eq!(hidden.created_at, time(1));
    assert_eq!(hidden.updated_at, time(2));
    assert_eq!(hidden.published_at, time(4));
    let visible = repo.get_notice_by_slug("new").await.unwrap().unwrap();
    assert!(!visible.hide_from_list);
    assert_eq!(visible.author_email.as_deref(), Some("author@example.com"));

    // Preserve NOW()'s DB clock and second precision at the publication boundary.
    sqlx::query("SET timestamp = 1893456000")
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE notices SET published_at = NOW() WHERE slug = 'future'")
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(repo.get_notice_by_slug("future").await.unwrap().is_some());
    sqlx::query("UPDATE notices SET published_at = DATE_ADD(NOW(), INTERVAL 1000 MICROSECOND) WHERE slug = 'future'").execute(&db.pool).await.unwrap();
    assert!(repo.get_notice_by_slug("future").await.unwrap().is_none());
}
