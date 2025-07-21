use async_trait::async_trait;
use eddist_core::domain::user_restriction::{RestrictionRuleType, UserRestrictionRule};
use sqlx::{MySql, Pool};
use uuid::Uuid;

#[async_trait]
pub trait UserRestrictionRepository: Send + Sync {
    async fn get_all_active_rules(&self) -> anyhow::Result<Vec<UserRestrictionRule>>;
}

#[derive(Clone)]
pub struct UserRestrictionRepositoryImpl {
    pool: Pool<MySql>,
}

impl UserRestrictionRepositoryImpl {
    pub fn new(pool: Pool<MySql>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRestrictionRepository for UserRestrictionRepositoryImpl {
    async fn get_all_active_rules(&self) -> anyhow::Result<Vec<UserRestrictionRule>> {
        let rows = sqlx::query!(
            r#"
            SELECT 
                BIN_TO_UUID(id) as id,
                name,
                rule_type,
                rule_value,
                expires_at,
                created_at,
                updated_at,
                created_by_email
            FROM user_restriction_rules
            WHERE expires_at IS NULL OR expires_at > NOW()
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut rules = Vec::with_capacity(rows.len());
        for row in rows {
            if let Some(id) = row.id {
                let rule_type = row
                    .rule_type
                    .parse::<RestrictionRuleType>()
                    .map_err(|e| anyhow::anyhow!("Invalid rule type '{}': {}", row.rule_type, e))?;

                rules.push(UserRestrictionRule {
                    id: Uuid::parse_str(&id)?,
                    name: row.name,
                    rule_type,
                    rule_value: row.rule_value,
                    expires_at: row.expires_at.map(|dt| dt.and_utc()),
                    created_at: row.created_at.and_utc(),
                    updated_at: row.updated_at.and_utc(),
                    created_by_email: row.created_by_email,
                });
            }
        }

        Ok(rules)
    }
}
