use async_trait::async_trait;
use eddist_core::domain::user_restriction::{RestrictionRuleType, UserRestrictionRule};
use eddist_entity::user_restriction;
use sea_orm::{
    ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    sea_query::{Expr, ExprTrait},
};

#[async_trait]
pub trait UserRestrictionRepository: Send + Sync {
    async fn get_all_active_rules(&self) -> anyhow::Result<Vec<UserRestrictionRule>>;
}

#[derive(Clone)]
pub struct UserRestrictionRepositoryImpl {
    db: DatabaseConnection,
}

impl UserRestrictionRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl UserRestrictionRepository for UserRestrictionRepositoryImpl {
    async fn get_all_active_rules(&self) -> anyhow::Result<Vec<UserRestrictionRule>> {
        let rows = user_restriction::Entity::find()
            .filter(
                Condition::any()
                    .add(user_restriction::Column::ExpiresAt.is_null())
                    .add(
                        Expr::col((
                            user_restriction::Entity,
                            user_restriction::Column::ExpiresAt,
                        ))
                        .gt(Expr::current_timestamp()),
                    ),
            )
            .order_by_desc(user_restriction::Column::CreatedAt)
            .all(&self.db)
            .await?;

        rows.into_iter()
            .map(|row| {
                let rule_type = row
                    .rule_type
                    .parse::<RestrictionRuleType>()
                    .map_err(|e| anyhow::anyhow!("Invalid rule type '{}': {}", row.rule_type, e))?;

                Ok(UserRestrictionRule {
                    id: row.id,
                    name: row.name,
                    rule_type,
                    target: row.target.parse().map_err(anyhow::Error::msg)?,
                    rule_value: row.rule_value,
                    expires_at: row.expires_at.map(|dt| dt.and_utc()),
                    created_at: row.created_at.and_utc(),
                    updated_at: row.updated_at.and_utc(),
                    created_by_email: row.created_by_email,
                })
            })
            .collect()
    }
}
