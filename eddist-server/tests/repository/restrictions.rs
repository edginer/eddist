use super::*;

#[tokio::test]
async fn restrictions_preserve_expiry_boundary_order_and_enum_mapping() {
    let db = TestDatabase::new().await;
    let repo = UserRestrictionRepositoryImpl::new(db.orm());
    assert!(repo.get_all_active_rules().await.unwrap().is_empty());
    sqlx::query("SET timestamp = 1893456000")
        .execute(&db.pool)
        .await
        .unwrap();
    let mut ids = Vec::new();
    for (day, expires_at, rule_type, target) in [
        (1, None, "IP", "BOTH"),
        (3, Some("2030-01-01 00:00:00.001"), "ASN", "POSTING"),
        (
            2,
            Some("2029-12-31 23:59:59.999"),
            "IP_CIDR",
            "AUTHENTICATION",
        ),
        (4, Some("2030-01-01 00:00:00.000"), "USER_AGENT", "BOTH"),
    ] {
        let id = Uuid::now_v7();
        sqlx::query("INSERT INTO user_restriction_rules (id, name, rule_type, target, rule_value, expires_at, created_at, updated_at, created_by_email) VALUES (UUID_TO_BIN(?), ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(id.to_string()).bind(format!("規制 {day}")).bind(rule_type).bind(target).bind("123")
            .bind(expires_at).bind(time(day)).bind(time(4)).bind("admin@example.com").execute(&db.pool).await.unwrap();
        ids.push(id);
    }
    let rules = repo.get_all_active_rules().await.unwrap();
    assert_eq!(
        rules.iter().map(|r| r.id).collect::<Vec<_>>(),
        [ids[1], ids[0]]
    );
    assert_eq!(rules[0].name, "規制 3");
    assert_eq!(rules[0].rule_type, RestrictionRuleType::Asn);
    assert_eq!(rules[0].target, RestrictionTarget::Posting);
    assert_eq!(rules[0].rule_value, "123");
    assert_eq!(rules[0].created_at, time(3).and_utc());
    assert_eq!(rules[0].updated_at, time(4).and_utc());
    assert_eq!(rules[0].created_by_email, "admin@example.com");
    assert_eq!(rules[0].expires_at.unwrap().timestamp_subsec_millis(), 1);
    assert_eq!(rules[1].rule_type, RestrictionRuleType::IP);
    assert_eq!(rules[1].target, RestrictionTarget::Both);
    assert!(rules[1].expires_at.is_none());
    sqlx::query("UPDATE user_restriction_rules SET target = 'invalid' WHERE id = UUID_TO_BIN(?)")
        .bind(ids[0].to_string())
        .execute(&db.pool)
        .await
        .unwrap();
    assert!(repo.get_all_active_rules().await.is_err());
    db.pool.close().await;
    assert!(repo.get_all_active_rules().await.is_err());
}
