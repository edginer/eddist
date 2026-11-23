use super::*;

#[tokio::test]
async fn captcha_preserves_active_filter_order_json_and_invalid_snapshot_handling() {
    let db = TestDatabase::new().await;
    let repo = CaptchaConfigRepositoryImpl::new(db.orm());
    assert!(repo.get_active_captcha_configs().await.unwrap().is_empty());
    for (name, order, day, active) in [
        ("second", 1, 2, true),
        ("first", 1, 1, true),
        ("last", 2, 1, true),
        ("inactive", 0, 1, false),
    ] {
        sqlx::query("INSERT INTO captcha_configs (id, name, provider, site_key, secret, capture_fields, is_active, display_order, endpoint_usage, created_at, updated_at) VALUES (UUID_TO_BIN(?), ?, 'turnstile', 'site-key', 'secret', ?, ?, ?, 'all', ?, ?)")
            .bind(Uuid::now_v7().to_string()).bind(name).bind(serde_json::json!(["hostname"]))
            .bind(active).bind(order).bind(time(day)).bind(time(day)).execute(&db.pool).await.unwrap();
    }
    let rows = repo.get_active_captcha_configs().await.unwrap();
    assert_eq!(
        rows.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
        ["first", "second", "last"]
    );
    assert_eq!(rows[0].capture_fields, ["hostname"]);
    assert_eq!(rows[0].site_key, "site-key");
    assert_eq!(rows[0].secret, "secret");
    assert!(rows[0].base_url.is_none());
    assert!(rows[0].verification.is_none());
    assert!(rows[0].endpoint_usage.matches_auth_code());
    assert!(rows[0].endpoint_usage.matches_reauth());
    assert!(
        rows[0]
            .widget
            .script_url
            .contains("challenges.cloudflare.com")
    );

    // Invalid JSON config is skipped without disabling otherwise valid providers.
    sqlx::query("UPDATE captcha_configs SET provider = 'custom', widget_form_field_name = 'answer', widget_script_url = '/widget.js', widget_html = '', base_url = 'https://captcha.example', verification = ?, capture_fields = NULL WHERE name = 'second'")
        .bind(serde_json::json!({"url":"{{base_url}}/verify"})).execute(&db.pool).await.unwrap();
    let rows = repo.get_active_captcha_configs().await.unwrap();
    assert_eq!(rows[1].widget.form_field_name, "answer");
    assert!(rows[1].capture_fields.is_empty());
    assert!(rows[1].verification.is_some());
    sqlx::query("UPDATE captcha_configs SET verification = ? WHERE name = 'second'")
        .bind(serde_json::json!({"headers":"invalid"}))
        .execute(&db.pool)
        .await
        .unwrap();
    assert_eq!(repo.get_active_captcha_configs().await.unwrap().len(), 2);
    sqlx::query("UPDATE captcha_configs SET secret = ''")
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(repo.get_active_captcha_configs().await.is_err());
    sqlx::query("UPDATE captcha_configs SET is_active = FALSE")
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(repo.get_active_captcha_configs().await.unwrap().is_empty());
    db.pool.close().await;
    assert!(repo.get_active_captcha_configs().await.is_err());
}
