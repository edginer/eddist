use async_trait::async_trait;
use eddist_core::domain::user_restriction::{
    CreateUserRestrictionRuleInput, RestrictionRuleType, UpdateUserRestrictionRuleInput,
    UserRestrictionRule,
};
#[cfg(not(feature = "backend-postgres"))]
use sqlx::{MySql, Pool};
use uuid::Uuid;

#[async_trait]
pub trait UserRestrictionRepository: Send + Sync {
    async fn get_all_rules(&self) -> anyhow::Result<Vec<UserRestrictionRule>>;
    async fn create_rule(
        &self,
        input: CreateUserRestrictionRuleInput,
    ) -> anyhow::Result<UserRestrictionRule>;
    async fn update_rule(&self, input: UpdateUserRestrictionRuleInput) -> anyhow::Result<()>;
    async fn delete_rule(&self, id: Uuid) -> anyhow::Result<()>;
    async fn get_rule_by_id(&self, id: Uuid) -> anyhow::Result<Option<UserRestrictionRule>>;
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
    async fn get_all_rules(&self) -> anyhow::Result<Vec<UserRestrictionRule>> {
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
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let mut rules = Vec::new();
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

    async fn create_rule(
        &self,
        input: CreateUserRestrictionRuleInput,
    ) -> anyhow::Result<UserRestrictionRule> {
        let id = Uuid::now_v7();
        let now = chrono::Utc::now().naive_utc();

        sqlx::query!(
            r#"
            INSERT INTO user_restriction_rules 
            (id, name, rule_type, rule_value, expires_at, created_at, updated_at, created_by_email)
            VALUES (UUID_TO_BIN(?), ?, ?, ?, ?, ?, ?, ?)
            "#,
            id.to_string(),
            input.name,
            input.rule_type.as_str(),
            input.rule_value,
            input.expires_at.map(|dt| dt.naive_utc()),
            now,
            now,
            input.created_by_email
        )
        .execute(&self.pool)
        .await?;

        Ok(UserRestrictionRule {
            id,
            name: input.name,
            rule_type: input.rule_type,
            rule_value: input.rule_value,
            expires_at: input.expires_at,
            created_at: now.and_utc(),
            updated_at: now.and_utc(),
            created_by_email: input.created_by_email,
        })
    }

    async fn update_rule(&self, input: UpdateUserRestrictionRuleInput) -> anyhow::Result<()> {
        let now = chrono::Utc::now().naive_utc();

        if let (Some(name), Some(rule_type), Some(rule_value)) =
            (&input.name, &input.rule_type, &input.rule_value)
        {
            sqlx::query!(
                r#"
                UPDATE user_restriction_rules 
                SET name = ?, rule_type = ?, rule_value = ?, expires_at = ?, updated_at = ?
                WHERE id = UUID_TO_BIN(?)
                "#,
                name,
                rule_type.as_str(),
                rule_value,
                input.expires_at.flatten().map(|dt| dt.naive_utc()),
                now,
                input.id.to_string()
            )
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    async fn delete_rule(&self, id: Uuid) -> anyhow::Result<()> {
        sqlx::query!(
            "DELETE FROM user_restriction_rules WHERE id = UUID_TO_BIN(?)",
            id.to_string()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_rule_by_id(&self, id: Uuid) -> anyhow::Result<Option<UserRestrictionRule>> {
        let row = sqlx::query!(
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
            WHERE id = UUID_TO_BIN(?)
            "#,
            id.to_string()
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row
            && let Some(id_str) = row.id
        {
            let rule_type = row
                .rule_type
                .parse::<RestrictionRuleType>()
                .map_err(|e| anyhow::anyhow!("Invalid rule type '{}': {}", row.rule_type, e))?;

            return Ok(Some(UserRestrictionRule {
                id: Uuid::parse_str(&id_str)?,
                name: row.name,
                rule_type,
                rule_value: row.rule_value,
                expires_at: row.expires_at.map(|dt| dt.and_utc()),
                created_at: row.created_at.and_utc(),
                updated_at: row.updated_at.and_utc(),
                created_by_email: row.created_by_email,
            }));
        }

        Ok(None)
    }
}

// PG row — TIMESTAMPTZ columns as DateTime<Utc>
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
    pool: sqlx::PgPool,
}

#[cfg(feature = "backend-postgres")]
impl UserRestrictionRepositoryPgImpl {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[cfg(feature = "backend-postgres")]
#[async_trait]
impl UserRestrictionRepository for UserRestrictionRepositoryPgImpl {
    async fn get_all_rules(&self) -> anyhow::Result<Vec<UserRestrictionRule>> {
        let rows = sqlx::query_as::<_, UserRestrictionRulePg>(
            r#"
            SELECT id, name, rule_type, rule_value, expires_at, created_at, updated_at, created_by_email
            FROM user_restriction_rules
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(UserRestrictionRule::try_from)
            .collect::<Result<Vec<_>, _>>()
    }

    async fn create_rule(
        &self,
        input: CreateUserRestrictionRuleInput,
    ) -> anyhow::Result<UserRestrictionRule> {
        let id = Uuid::now_v7();
        let now = chrono::Utc::now();

        sqlx::query(
            r#"
            INSERT INTO user_restriction_rules
            (id, name, rule_type, rule_value, expires_at, created_at, updated_at, created_by_email)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(id)
        .bind(&input.name)
        .bind(input.rule_type.as_str())
        .bind(&input.rule_value)
        .bind(input.expires_at)
        .bind(now)
        .bind(now)
        .bind(&input.created_by_email)
        .execute(&self.pool)
        .await?;

        Ok(UserRestrictionRule {
            id,
            name: input.name,
            rule_type: input.rule_type,
            rule_value: input.rule_value,
            expires_at: input.expires_at,
            created_at: now,
            updated_at: now,
            created_by_email: input.created_by_email,
        })
    }

    async fn update_rule(&self, input: UpdateUserRestrictionRuleInput) -> anyhow::Result<()> {
        let now = chrono::Utc::now();

        if let (Some(name), Some(rule_type), Some(rule_value)) =
            (&input.name, &input.rule_type, &input.rule_value)
        {
            sqlx::query(
                r#"
                UPDATE user_restriction_rules
                SET name = $1, rule_type = $2, rule_value = $3, expires_at = $4, updated_at = $5
                WHERE id = $6
                "#,
            )
            .bind(name)
            .bind(rule_type.as_str())
            .bind(rule_value)
            .bind(input.expires_at.flatten())
            .bind(now)
            .bind(input.id)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    async fn delete_rule(&self, id: Uuid) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM user_restriction_rules WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_rule_by_id(&self, id: Uuid) -> anyhow::Result<Option<UserRestrictionRule>> {
        let row = sqlx::query_as::<_, UserRestrictionRulePg>(
            r#"
            SELECT id, name, rule_type, rule_value, expires_at, created_at, updated_at, created_by_email
            FROM user_restriction_rules
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(UserRestrictionRule::try_from).transpose()
    }
}
