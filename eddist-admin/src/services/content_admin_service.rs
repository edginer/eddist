use std::sync::Arc;

use uuid::Uuid;

use crate::{
    auth::AdminIdentity,
    models::{
        CaptchaConfig, CreateCaptchaConfigInput, Notice, Terms, UpdateCaptchaConfigInput,
        idp::{CreateIdpInput, Idp, UpdateIdpInput},
        server_settings::{ServerSetting, UpsertServerSettingInput},
    },
    repository::{
        captcha_config_repository::CaptchaConfigRepository,
        idp_repository::IdpAdminRepository,
        notice_repository::{CreateNoticeInput, NoticeRepository, UpdateNoticeInput},
        server_settings_repository::ServerSettingsRepository,
        terms_repository::{TermsRepository, UpdateTermsInput},
    },
};

#[async_trait::async_trait]
pub trait ContentAdminService: Send + Sync {
    // Notices
    async fn get_notices(&self, page: u32, limit: u32) -> anyhow::Result<Vec<Notice>>;
    async fn get_notice(&self, id: Uuid) -> anyhow::Result<Option<Notice>>;
    async fn create_notice(
        &self,
        actor: &AdminIdentity,
        input: CreateNoticeInput,
    ) -> anyhow::Result<Notice>;
    async fn update_notice(
        &self,
        actor: &AdminIdentity,
        id: Uuid,
        input: UpdateNoticeInput,
    ) -> anyhow::Result<Notice>;
    async fn delete_notice(&self, actor: &AdminIdentity, id: Uuid) -> anyhow::Result<()>;
    async fn check_notice_author(
        &self,
        actor: &AdminIdentity,
        id: Uuid,
    ) -> anyhow::Result<eddist_core::domain::notice::Notice>;
    // Terms
    async fn get_terms(&self) -> anyhow::Result<Option<Terms>>;
    async fn update_terms(
        &self,
        actor: &AdminIdentity,
        input: UpdateTermsInput,
    ) -> anyhow::Result<Terms>;
    // Server settings
    async fn list_server_settings(&self) -> anyhow::Result<Vec<ServerSetting>>;
    async fn upsert_server_setting(
        &self,
        actor: &AdminIdentity,
        input: UpsertServerSettingInput,
    ) -> anyhow::Result<ServerSetting>;
    // IdPs
    async fn list_idps(&self) -> anyhow::Result<Vec<Idp>>;
    async fn get_idp(&self, id: Uuid) -> anyhow::Result<Option<Idp>>;
    async fn create_idp(&self, actor: &AdminIdentity, input: CreateIdpInput)
    -> anyhow::Result<Idp>;
    async fn update_idp(
        &self,
        actor: &AdminIdentity,
        id: Uuid,
        input: UpdateIdpInput,
    ) -> anyhow::Result<Idp>;
    async fn delete_idp(&self, actor: &AdminIdentity, id: Uuid) -> anyhow::Result<()>;
    // Captcha configs
    async fn list_captcha_configs(&self) -> anyhow::Result<Vec<CaptchaConfig>>;
    async fn get_captcha_config(&self, id: Uuid) -> anyhow::Result<Option<CaptchaConfig>>;
    async fn create_captcha_config(
        &self,
        actor: &AdminIdentity,
        input: CreateCaptchaConfigInput,
    ) -> anyhow::Result<CaptchaConfig>;
    async fn update_captcha_config(
        &self,
        actor: &AdminIdentity,
        id: Uuid,
        input: UpdateCaptchaConfigInput,
    ) -> anyhow::Result<CaptchaConfig>;
    async fn delete_captcha_config(&self, actor: &AdminIdentity, id: Uuid) -> anyhow::Result<()>;
}

pub struct ContentAdminServiceImpl {
    notice_repo: Arc<dyn NoticeRepository>,
    terms_repo: Arc<dyn TermsRepository>,
    server_settings_repo: Arc<dyn ServerSettingsRepository>,
    idp_repo: Arc<dyn IdpAdminRepository>,
    captcha_config_repo: Arc<dyn CaptchaConfigRepository>,
}

impl ContentAdminServiceImpl {
    pub fn new(
        notice_repo: Arc<dyn NoticeRepository>,
        terms_repo: Arc<dyn TermsRepository>,
        server_settings_repo: Arc<dyn ServerSettingsRepository>,
        idp_repo: Arc<dyn IdpAdminRepository>,
        captcha_config_repo: Arc<dyn CaptchaConfigRepository>,
    ) -> Self {
        Self {
            notice_repo,
            terms_repo,
            server_settings_repo,
            idp_repo,
            captcha_config_repo,
        }
    }
}

#[async_trait::async_trait]
impl ContentAdminService for ContentAdminServiceImpl {
    async fn get_notices(&self, page: u32, limit: u32) -> anyhow::Result<Vec<Notice>> {
        let notices = self.notice_repo.get_notices_paginated(page, limit).await?;
        Ok(notices.into_iter().map(Notice::from).collect())
    }

    async fn get_notice(&self, id: Uuid) -> anyhow::Result<Option<Notice>> {
        Ok(self
            .notice_repo
            .get_notice_by_id(id)
            .await?
            .map(Notice::from))
    }

    async fn create_notice(
        &self,
        actor: &AdminIdentity,
        input: CreateNoticeInput,
    ) -> anyhow::Result<Notice> {
        let notice = self
            .notice_repo
            .create_notice(input, Some(actor.email.clone()))
            .await?;
        Ok(notice.into())
    }

    async fn update_notice(
        &self,
        _actor: &AdminIdentity,
        id: Uuid,
        input: UpdateNoticeInput,
    ) -> anyhow::Result<Notice> {
        let notice = self.notice_repo.update_notice(id, input).await?;
        Ok(notice.into())
    }

    async fn delete_notice(&self, _actor: &AdminIdentity, id: Uuid) -> anyhow::Result<()> {
        self.notice_repo.delete_notice(id).await
    }

    async fn check_notice_author(
        &self,
        actor: &AdminIdentity,
        id: Uuid,
    ) -> anyhow::Result<eddist_core::domain::notice::Notice> {
        let notice = self
            .notice_repo
            .get_notice_by_id(id)
            .await?
            .ok_or_else(|| crate::error::ServiceError::NotFound("Notice not found".into()))?;
        if notice.author_email.as_ref() != Some(&actor.email) {
            return Err(crate::error::ServiceError::Forbidden(
                "you can only modify notices you created".into(),
            )
            .into());
        }
        Ok(notice)
    }

    async fn get_terms(&self) -> anyhow::Result<Option<Terms>> {
        Ok(self.terms_repo.get_terms().await?.map(Terms::from))
    }

    async fn update_terms(
        &self,
        actor: &AdminIdentity,
        input: UpdateTermsInput,
    ) -> anyhow::Result<Terms> {
        let terms = self
            .terms_repo
            .update_terms(input, Some(actor.email.clone()))
            .await?;
        Ok(terms.into())
    }

    async fn list_server_settings(&self) -> anyhow::Result<Vec<ServerSetting>> {
        self.server_settings_repo.get_all().await
    }

    async fn upsert_server_setting(
        &self,
        _actor: &AdminIdentity,
        input: UpsertServerSettingInput,
    ) -> anyhow::Result<ServerSetting> {
        self.server_settings_repo.upsert(input).await
    }

    async fn list_idps(&self) -> anyhow::Result<Vec<Idp>> {
        self.idp_repo.get_all().await
    }

    async fn get_idp(&self, id: Uuid) -> anyhow::Result<Option<Idp>> {
        self.idp_repo.get_by_id(id).await
    }

    async fn create_idp(
        &self,
        _actor: &AdminIdentity,
        input: CreateIdpInput,
    ) -> anyhow::Result<Idp> {
        self.idp_repo.create(input).await
    }

    async fn update_idp(
        &self,
        _actor: &AdminIdentity,
        id: Uuid,
        input: UpdateIdpInput,
    ) -> anyhow::Result<Idp> {
        self.idp_repo.update(id, input).await
    }

    async fn delete_idp(&self, _actor: &AdminIdentity, id: Uuid) -> anyhow::Result<()> {
        self.idp_repo.delete(id).await
    }

    async fn list_captcha_configs(&self) -> anyhow::Result<Vec<CaptchaConfig>> {
        self.captcha_config_repo.get_all().await
    }

    async fn get_captcha_config(&self, id: Uuid) -> anyhow::Result<Option<CaptchaConfig>> {
        self.captcha_config_repo.get_by_id(id).await
    }

    async fn create_captcha_config(
        &self,
        actor: &AdminIdentity,
        input: CreateCaptchaConfigInput,
    ) -> anyhow::Result<CaptchaConfig> {
        self.captcha_config_repo
            .create(input, Some(actor.email.clone()))
            .await
    }

    async fn update_captcha_config(
        &self,
        actor: &AdminIdentity,
        id: Uuid,
        input: UpdateCaptchaConfigInput,
    ) -> anyhow::Result<CaptchaConfig> {
        self.captcha_config_repo
            .update(id, input, Some(actor.email.clone()))
            .await
    }

    async fn delete_captcha_config(&self, _actor: &AdminIdentity, id: Uuid) -> anyhow::Result<()> {
        self.captcha_config_repo.delete(id).await
    }
}
