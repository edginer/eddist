use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::{query_as, MySqlPool};
use uuid::Uuid;

use crate::domain::service::user_restriction_service::{RestrictionType, UserRestrictionRule};

#[async_trait::async_trait]
pub trait UserRestrictionRepository: Send + Sync + 'static {
    async fn get_active_user_restriction_rules_by_type(
        &self,
        restriction_type: RestrictionType,
    ) -> anyhow::Result<Vec<UserRestrictionRule>>;
}

#[derive(Debug, Clone)]
pub struct UserRestrictionRepositoryImpl {
    pool: MySqlPool,
}

impl UserRestrictionRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl UserRestrictionRepository for UserRestrictionRepositoryImpl {
    async fn get_active_user_restriction_rules_by_type(
        &self,
        restriction_type: RestrictionType,
    ) -> anyhow::Result<Vec<UserRestrictionRule>> {
        let rules = query_as!(
            UserRestrictionRuleRow,
            r#"
            SELECT id, name, filter_expression, restriction_type, active, created_at, updated_at, created_by, description
            FROM user_restriction_rules
            WHERE active = true AND restriction_type = ?
            ORDER BY created_at DESC
            "#,
            restriction_type.as_str()
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rules.into_iter().map(Into::into).collect())
    }
}

#[derive(Debug)]
struct UserRestrictionRuleRow {
    id: Vec<u8>,
    name: String,
    filter_expression: String,
    restriction_type: String,
    active: i8,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    created_by: Option<String>,
    description: Option<String>,
}

impl From<UserRestrictionRuleRow> for UserRestrictionRule {
    fn from(row: UserRestrictionRuleRow) -> Self {
        Self {
            id: Uuid::from_slice(&row.id).unwrap(),
            name: row.name,
            filter_expression: row.filter_expression,
            restriction_type: RestrictionType::from_str(&row.restriction_type)
                .unwrap_or(RestrictionType::CreatingResponse),
            active: row.active != 0,
            created_at: DateTime::from_naive_utc_and_offset(row.created_at, Utc),
            updated_at: DateTime::from_naive_utc_and_offset(row.updated_at, Utc),
            created_by: row.created_by,
            description: row.description,
        }
    }
}
