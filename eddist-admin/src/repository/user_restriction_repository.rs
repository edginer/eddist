use crate::entity::user_restriction;
use async_trait::async_trait;
use eddist_core::domain::user_restriction::{
    CreateUserRestrictionRuleInput, RestrictionRuleType, UpdateUserRestrictionRuleInput,
    UserRestrictionRule,
};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait, QueryOrder};
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

#[derive(Clone)]
pub struct UserRestrictionRepositoryImpl(DatabaseConnection);

impl UserRestrictionRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self(db)
    }
}

fn into_domain(model: user_restriction::Model) -> anyhow::Result<UserRestrictionRule> {
    let rule_type = model
        .rule_type
        .parse::<RestrictionRuleType>()
        .map_err(|error| anyhow::anyhow!("Invalid restriction rule type: {error}"))?;

    Ok(UserRestrictionRule {
        id: model.id,
        name: model.name,
        rule_type,
        rule_value: model.rule_value,
        expires_at: model.expires_at.map(|date_time| date_time.and_utc()),
        created_at: model.created_at.and_utc(),
        updated_at: model.updated_at.and_utc(),
        created_by_email: model.created_by_email,
    })
}

#[async_trait]
impl UserRestrictionRepository for UserRestrictionRepositoryImpl {
    async fn get_all_rules(&self) -> anyhow::Result<Vec<UserRestrictionRule>> {
        user_restriction::Entity::find()
            .order_by_desc(user_restriction::Column::CreatedAt)
            .all(&self.0)
            .await?
            .into_iter()
            .map(into_domain)
            .collect()
    }

    async fn create_rule(
        &self,
        input: CreateUserRestrictionRuleInput,
    ) -> anyhow::Result<UserRestrictionRule> {
        let id = Uuid::now_v7();
        let now = crate::db_time::now();
        let model = user_restriction::ActiveModel {
            id: Set(id),
            name: Set(input.name),
            rule_type: Set(input.rule_type.as_str().to_string()),
            rule_value: Set(input.rule_value),
            expires_at: Set(input
                .expires_at
                .map(|date_time| crate::db_time::truncate_to_millis(date_time.naive_utc()))),
            created_at: Set(now),
            updated_at: Set(now),
            created_by_email: Set(input.created_by_email),
        }
        .insert(&self.0)
        .await?;

        into_domain(model)
    }

    async fn update_rule(&self, input: UpdateUserRestrictionRuleInput) -> anyhow::Result<()> {
        let now = crate::db_time::now();
        let current = self
            .get_rule_by_id(input.id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("restriction rule not found: {}", input.id))?;

        let name = input.name.unwrap_or(current.name);
        let rule_type = input.rule_type.unwrap_or(current.rule_type);
        let rule_value = input.rule_value.unwrap_or(current.rule_value);
        let expires_at = input.expires_at.unwrap_or(current.expires_at);

        user_restriction::ActiveModel {
            id: Set(input.id),
            name: Set(name),
            rule_type: Set(rule_type.as_str().to_string()),
            rule_value: Set(rule_value),
            expires_at: Set(expires_at
                .map(|date_time| crate::db_time::truncate_to_millis(date_time.naive_utc()))),
            updated_at: Set(now),
            ..Default::default()
        }
        .update(&self.0)
        .await?;

        Ok(())
    }

    async fn delete_rule(&self, id: Uuid) -> anyhow::Result<()> {
        user_restriction::Entity::delete_by_id(id)
            .exec(&self.0)
            .await?;
        Ok(())
    }

    async fn get_rule_by_id(&self, id: Uuid) -> anyhow::Result<Option<UserRestrictionRule>> {
        user_restriction::Entity::find_by_id(id)
            .one(&self.0)
            .await?
            .map(into_domain)
            .transpose()
    }
}
