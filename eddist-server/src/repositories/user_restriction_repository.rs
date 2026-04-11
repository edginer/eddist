use async_trait::async_trait;
use eddist_core::domain::user_restriction::{RestrictionRuleType, UserRestrictionRule};
#[cfg(feature = "backend-postgres")]
use sqlx::PgPool;
#[cfg(not(feature = "backend-postgres"))]
use sqlx::{MySql, Pool};
use uuid::Uuid;

#[async_trait]
pub trait UserRestrictionRepository: Send + Sync {
    async fn get_all_active_rules(&self) -> anyhow::Result<Vec<UserRestrictionRule>>;
}

#[cfg(not(feature = "backend-postgres"))]
#[derive(Clone)]
pub struct UserRestrictionRepositoryImpl {
    pool: Pool<MySql>,
}

#[cfg(not(feature = "backend-postgres"))]
impl UserRestrictionRepositoryImpl {
    pub fn new(pool: Pool<MySql>) -> Self {
        Self { pool }
    }
}

#[cfg(not(feature = "backend-postgres"))]
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

#[cfg(feature = "backend-postgres")]
#[derive(Debug, sqlx::FromRow)]
struct UserRestrictionRulePg {
    pub id: Uuid,
    pub name: String,
    pub rule_type: String,
    pub rule_value: String,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub created_by_email: String,
}

#[cfg(feature = "backend-postgres")]
impl TryFrom<UserRestrictionRulePg> for UserRestrictionRule {
    type Error = anyhow::Error;
    fn try_from(r: UserRestrictionRulePg) -> Result<Self, Self::Error> {
        let rule_type = r
            .rule_type
            .parse::<RestrictionRuleType>()
            .map_err(|e| anyhow::anyhow!("Invalid rule type '{}': {}", r.rule_type, e))?;
        Ok(UserRestrictionRule {
            id: r.id,
            name: r.name,
            rule_type,
            rule_value: r.rule_value,
            expires_at: r.expires_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
            created_by_email: r.created_by_email,
        })
    }
}

#[cfg(feature = "backend-postgres")]
#[derive(Clone)]
pub struct UserRestrictionRepositoryPgImpl {
    pool: PgPool,
}

#[cfg(feature = "backend-postgres")]
impl UserRestrictionRepositoryPgImpl {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[cfg(feature = "backend-postgres")]
#[async_trait]
impl UserRestrictionRepository for UserRestrictionRepositoryPgImpl {
    async fn get_all_active_rules(&self) -> anyhow::Result<Vec<UserRestrictionRule>> {
        let rows = sqlx::query_as::<_, UserRestrictionRulePg>(
            r#"
            SELECT id, name, rule_type, rule_value, expires_at, created_at, updated_at, created_by_email
            FROM user_restriction_rules
            WHERE expires_at IS NULL OR expires_at > NOW()
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(UserRestrictionRule::try_from)
            .collect::<Result<Vec<_>, _>>()
    }
}
