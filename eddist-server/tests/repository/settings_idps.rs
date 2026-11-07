use super::*;

#[tokio::test]
async fn settings_and_idps_preserve_values_and_disabled_providers() {
    let db = TestDatabase::new().await;
    let settings = ServerSettingsRepositoryImpl::new(db.orm());
    assert!(
        settings
            .get_all()
            .await
            .unwrap()
            .get(ServerSettingKey::CloseNewAuthentication)
            .is_none()
    );
    sqlx::query("INSERT INTO server_settings (id, setting_key, value, created_at, updated_at) VALUES (UUID_TO_BIN(?), 'bbs.close_new_authentication', 'true', ?, ?)")
        .bind(Uuid::now_v7().to_string()).bind(time(1)).bind(time(1)).execute(&db.pool).await.unwrap();
    assert!(
        settings
            .get_all()
            .await
            .unwrap()
            .is_enabled(ServerSettingKey::CloseNewAuthentication)
    );
    sqlx::query("UPDATE server_settings SET value = 'false'")
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(
        !settings
            .get_all()
            .await
            .unwrap()
            .is_enabled(ServerSettingKey::CloseNewAuthentication)
    );

    let idps = IdpRepositoryImpl::new(db.orm());
    assert!(idps.get_idps().await.unwrap().is_empty());
    let mut ids = Vec::new();
    for (name, enabled, logo) in [("enabled", true, Some("<svg/>")), ("disabled", false, None)] {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO idps (id, idp_name, idp_display_name, idp_logo_svg, oidc_config_url, client_id, client_secret, enabled) VALUES (UUID_TO_BIN(?), ?, ?, ?, ?, ?, ?, ?)")
            .bind(id.to_string()).bind(name).bind(format!("認証 {name}")).bind(logo)
            .bind("https://idp.example/.well-known/openid-configuration").bind("client-id")
            .bind("encrypted-secret").bind(enabled).execute(&db.pool).await.unwrap();
        ids.push(id);
    }
    let mut rows = idps.get_idps().await.unwrap();
    rows.sort_by_key(|idp| !idp.enabled);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].id, ids[0]);
    assert_eq!(rows[1].id, ids[1]);
    assert_eq!(rows[0].idp_name, "enabled");
    assert_eq!(rows[0].idp_display_name, "認証 enabled");
    assert_eq!(rows[0].idp_logo_svg.as_deref(), Some("<svg/>"));
    assert!(rows[1].idp_logo_svg.is_none());
    assert_eq!(
        rows[0].oidc_config_url,
        "https://idp.example/.well-known/openid-configuration"
    );
    assert_eq!(rows[0].client_id, "client-id");
    assert_eq!(rows[0].client_secret, "encrypted-secret");
    assert!(!rows[1].enabled);
    db.pool.close().await;
    assert!(settings.get_all().await.is_err());
    assert!(idps.get_idps().await.is_err());
}
