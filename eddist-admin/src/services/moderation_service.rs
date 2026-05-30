use std::sync::Arc;

use chrono::{DateTime, Utc};
use eddist_core::domain::user_restriction::{
    CreateUserRestrictionRuleInput, UpdateUserRestrictionRuleInput, UserRestrictionRule,
};
use uuid::Uuid;

use crate::{
    auth::AdminIdentity,
    models::{
        Cap, CreationCapInput, CreationNgWordInput, NgWord, UpdateCapInput, UpdateNgWordInput,
    },
    repository::{
        cap_repository::CapRepository, ngword_repository::NgWordRepository,
        user_restriction_repository::UserRestrictionRepository,
    },
};

#[async_trait::async_trait]
pub trait ModerationService: Send + Sync {
    // NG words
    async fn get_ng_words(&self) -> anyhow::Result<Vec<NgWord>>;
    async fn create_ng_word(
        &self,
        actor: &AdminIdentity,
        input: CreationNgWordInput,
    ) -> anyhow::Result<NgWord>;
    async fn update_ng_word(
        &self,
        actor: &AdminIdentity,
        id: Uuid,
        input: UpdateNgWordInput,
    ) -> anyhow::Result<NgWord>;
    async fn delete_ng_word(&self, actor: &AdminIdentity, id: Uuid) -> anyhow::Result<()>;
    // Caps
    async fn get_caps(&self) -> anyhow::Result<Vec<Cap>>;
    async fn create_cap(
        &self,
        actor: &AdminIdentity,
        input: CreationCapInput,
    ) -> anyhow::Result<Cap>;
    async fn update_cap(
        &self,
        actor: &AdminIdentity,
        id: Uuid,
        input: UpdateCapInput,
    ) -> anyhow::Result<Cap>;
    async fn delete_cap(&self, actor: &AdminIdentity, id: Uuid) -> anyhow::Result<()>;
    // Restriction rules
    async fn get_restriction_rules(&self) -> anyhow::Result<Vec<UserRestrictionRule>>;
    async fn get_restriction_rule(&self, id: Uuid) -> anyhow::Result<Option<UserRestrictionRule>>;
    async fn create_restriction_rule(
        &self,
        actor: &AdminIdentity,
        name: String,
        rule_type: eddist_core::domain::user_restriction::RestrictionRuleType,
        rule_value: String,
        expires_at: Option<DateTime<Utc>>,
    ) -> anyhow::Result<UserRestrictionRule>;
    async fn update_restriction_rule(
        &self,
        actor: &AdminIdentity,
        input: UpdateUserRestrictionRuleInput,
    ) -> anyhow::Result<()>;
    async fn delete_restriction_rule(&self, actor: &AdminIdentity, id: Uuid) -> anyhow::Result<()>;
}

pub struct ModerationServiceImpl {
    ng_word_repo: Arc<dyn NgWordRepository>,
    cap_repo: Arc<dyn CapRepository>,
    user_restriction_repo: Arc<dyn UserRestrictionRepository>,
}

impl ModerationServiceImpl {
    pub fn new(
        ng_word_repo: Arc<dyn NgWordRepository>,
        cap_repo: Arc<dyn CapRepository>,
        user_restriction_repo: Arc<dyn UserRestrictionRepository>,
    ) -> Self {
        Self {
            ng_word_repo,
            cap_repo,
            user_restriction_repo,
        }
    }
}

#[async_trait::async_trait]
impl ModerationService for ModerationServiceImpl {
    async fn get_ng_words(&self) -> anyhow::Result<Vec<NgWord>> {
        self.ng_word_repo.get_ng_words().await
    }

    async fn create_ng_word(
        &self,
        _actor: &AdminIdentity,
        input: CreationNgWordInput,
    ) -> anyhow::Result<NgWord> {
        self.ng_word_repo
            .create_ng_word(&input.name, &input.word)
            .await
    }

    async fn update_ng_word(
        &self,
        _actor: &AdminIdentity,
        id: Uuid,
        input: UpdateNgWordInput,
    ) -> anyhow::Result<NgWord> {
        self.ng_word_repo
            .update_ng_word(
                id,
                input.name.as_deref(),
                input.word.as_deref(),
                input.board_ids,
            )
            .await
    }

    async fn delete_ng_word(&self, _actor: &AdminIdentity, id: Uuid) -> anyhow::Result<()> {
        self.ng_word_repo.delete_ng_word(id).await
    }

    async fn get_caps(&self) -> anyhow::Result<Vec<Cap>> {
        self.cap_repo.get_caps().await
    }

    async fn create_cap(
        &self,
        _actor: &AdminIdentity,
        input: CreationCapInput,
    ) -> anyhow::Result<Cap> {
        let tinker_secret = std::env::var("TINKER_SECRET")
            .map_err(|_| anyhow::anyhow!("TINKER_SECRET not configured"))?;
        self.cap_repo
            .create_cap(
                &input.name,
                &input.description,
                &eddist_core::domain::cap::calculate_cap_hash(&input.password, &tinker_secret),
            )
            .await
    }

    async fn update_cap(
        &self,
        _actor: &AdminIdentity,
        id: Uuid,
        input: UpdateCapInput,
    ) -> anyhow::Result<Cap> {
        let hashed_password = input.password.map(|p| {
            let secret = std::env::var("TINKER_SECRET").unwrap_or_default();
            eddist_core::domain::cap::calculate_cap_hash(&p, &secret)
        });
        self.cap_repo
            .update_cap(
                id,
                input.name.as_deref(),
                input.description.as_deref(),
                hashed_password.as_deref(),
                input.board_ids,
            )
            .await
    }

    async fn delete_cap(&self, _actor: &AdminIdentity, id: Uuid) -> anyhow::Result<()> {
        self.cap_repo.delete_cap(id).await
    }

    async fn get_restriction_rules(&self) -> anyhow::Result<Vec<UserRestrictionRule>> {
        Ok(self
            .user_restriction_repo
            .get_all_rules()
            .await
            .unwrap_or_default())
    }

    async fn get_restriction_rule(&self, id: Uuid) -> anyhow::Result<Option<UserRestrictionRule>> {
        self.user_restriction_repo.get_rule_by_id(id).await
    }

    async fn create_restriction_rule(
        &self,
        actor: &AdminIdentity,
        name: String,
        rule_type: eddist_core::domain::user_restriction::RestrictionRuleType,
        rule_value: String,
        expires_at: Option<DateTime<Utc>>,
    ) -> anyhow::Result<UserRestrictionRule> {
        let input = CreateUserRestrictionRuleInput {
            name,
            rule_type,
            rule_value,
            expires_at,
            created_by_email: actor.email.clone(),
        };
        self.user_restriction_repo.create_rule(input).await
    }

    async fn update_restriction_rule(
        &self,
        _actor: &AdminIdentity,
        input: UpdateUserRestrictionRuleInput,
    ) -> anyhow::Result<()> {
        self.user_restriction_repo.update_rule(input).await
    }

    async fn delete_restriction_rule(
        &self,
        _actor: &AdminIdentity,
        id: Uuid,
    ) -> anyhow::Result<()> {
        self.user_restriction_repo.delete_rule(id).await
    }
}
