use crate::entity::captcha_config;
use crate::repository::support::empty_to_none;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder,
};
use uuid::Uuid;

use crate::models::{
    CaptchaConfig, CaptchaVerificationConfig, CaptchaWidgetConfig, CreateCaptchaConfigInput,
    UpdateCaptchaConfigInput,
};

fn into_domain(model: captcha_config::Model) -> CaptchaConfig {
    let capture_fields: Vec<String> = model
        .capture_fields
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default();

    let verification: Option<CaptchaVerificationConfig> = model
        .verification
        .and_then(|value| serde_json::from_value(value).ok());

    // Widget is only present if all required fields are set.
    let widget = match (
        model.widget_form_field_name,
        model.widget_script_url,
        model.widget_html,
    ) {
        (Some(form_field_name), Some(script_url), Some(widget_html)) => Some(CaptchaWidgetConfig {
            form_field_name,
            script_url,
            widget_html,
            script_handler: model.widget_script_handler,
        }),
        _ => None,
    };

    CaptchaConfig {
        id: model.id,
        name: model.name,
        provider: model.provider,
        site_key: model.site_key,
        secret: model.secret,
        base_url: model.base_url,
        widget,
        capture_fields,
        verification,
        is_active: model.is_active,
        display_order: model.display_order,
        endpoint_usage: model.endpoint_usage,
        created_at: model.created_at,
        updated_at: model.updated_at,
        updated_by: model.updated_by,
    }
}

#[async_trait::async_trait]
pub trait CaptchaConfigRepository: Send + Sync {
    async fn get_all(&self) -> anyhow::Result<Vec<CaptchaConfig>>;
    async fn get_active(&self) -> anyhow::Result<Vec<CaptchaConfig>>;
    async fn get_by_id(&self, id: Uuid) -> anyhow::Result<Option<CaptchaConfig>>;
    async fn create(
        &self,
        input: CreateCaptchaConfigInput,
        updated_by: Option<String>,
    ) -> anyhow::Result<CaptchaConfig>;
    async fn update(
        &self,
        id: Uuid,
        input: UpdateCaptchaConfigInput,
        updated_by: Option<String>,
    ) -> anyhow::Result<CaptchaConfig>;
    async fn delete(&self, id: Uuid) -> anyhow::Result<()>;
}

#[derive(Clone)]
pub struct CaptchaConfigRepositoryImpl(DatabaseConnection);

impl CaptchaConfigRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self(db)
    }
}

#[async_trait::async_trait]
impl CaptchaConfigRepository for CaptchaConfigRepositoryImpl {
    async fn get_all(&self) -> anyhow::Result<Vec<CaptchaConfig>> {
        Ok(captcha_config::Entity::find()
            .order_by_asc(captcha_config::Column::DisplayOrder)
            .order_by_asc(captcha_config::Column::CreatedAt)
            .all(&self.0)
            .await?
            .into_iter()
            .map(into_domain)
            .collect())
    }

    async fn get_active(&self) -> anyhow::Result<Vec<CaptchaConfig>> {
        Ok(captcha_config::Entity::find()
            .filter(captcha_config::Column::IsActive.eq(true))
            .order_by_asc(captcha_config::Column::DisplayOrder)
            .order_by_asc(captcha_config::Column::CreatedAt)
            .all(&self.0)
            .await?
            .into_iter()
            .map(into_domain)
            .collect())
    }

    async fn get_by_id(&self, id: Uuid) -> anyhow::Result<Option<CaptchaConfig>> {
        Ok(captcha_config::Entity::find_by_id(id)
            .one(&self.0)
            .await?
            .map(into_domain))
    }

    async fn create(
        &self,
        input: CreateCaptchaConfigInput,
        updated_by: Option<String>,
    ) -> anyhow::Result<CaptchaConfig> {
        let id = Uuid::now_v7();
        let now = crate::db_time::now();

        let capture_fields = serde_json::to_value(&input.capture_fields)?;
        let verification = input
            .verification
            .as_ref()
            .map(serde_json::to_value)
            .transpose()?;
        let (widget_form_field_name, widget_script_url, widget_html, widget_script_handler) =
            widget_columns(input.widget.as_ref());

        let model = captcha_config::ActiveModel {
            id: Set(id),
            name: Set(input.name),
            provider: Set(input.provider),
            site_key: Set(input.site_key),
            secret: Set(input.secret),
            base_url: Set(input.base_url),
            widget_form_field_name: Set(widget_form_field_name),
            widget_script_url: Set(widget_script_url),
            widget_html: Set(widget_html),
            widget_script_handler: Set(widget_script_handler),
            capture_fields: Set(Some(capture_fields)),
            verification: Set(verification),
            is_active: Set(input.is_active),
            display_order: Set(input.display_order),
            endpoint_usage: Set(input.endpoint_usage),
            created_at: Set(now),
            updated_at: Set(now),
            updated_by: Set(updated_by),
        }
        .insert(&self.0)
        .await?;

        Ok(into_domain(model))
    }

    async fn update(
        &self,
        id: Uuid,
        input: UpdateCaptchaConfigInput,
        updated_by: Option<String>,
    ) -> anyhow::Result<CaptchaConfig> {
        let now = crate::db_time::now();
        let current = self.get_by_id(id).await?.ok_or_else(|| {
            crate::error::ServiceError::NotFound("Captcha config not found".into())
        })?;

        let name = input.name.unwrap_or(current.name);
        let provider = input.provider.unwrap_or(current.provider);
        let site_key = input.site_key.unwrap_or(current.site_key);
        let secret = input
            .secret
            .filter(|value| !value.is_empty())
            .unwrap_or(current.secret);
        let base_url = input.base_url.or(current.base_url);
        let widget = input.widget.or(current.widget);
        let capture_fields = input.capture_fields.unwrap_or(current.capture_fields);
        let verification = input.verification.or(current.verification);
        let is_active = input.is_active.unwrap_or(current.is_active);
        let display_order = input.display_order.unwrap_or(current.display_order);
        let endpoint_usage = input.endpoint_usage.unwrap_or(current.endpoint_usage);

        let capture_fields_json = serde_json::to_value(&capture_fields)?;
        let verification_json = verification
            .as_ref()
            .map(serde_json::to_value)
            .transpose()?;
        let (widget_form_field_name, widget_script_url, widget_html, widget_script_handler) =
            widget_columns(widget.as_ref());

        let model = captcha_config::ActiveModel {
            id: Set(id),
            name: Set(name),
            provider: Set(provider),
            site_key: Set(site_key),
            secret: Set(secret),
            base_url: Set(base_url),
            widget_form_field_name: Set(widget_form_field_name),
            widget_script_url: Set(widget_script_url),
            widget_html: Set(widget_html),
            widget_script_handler: Set(widget_script_handler),
            capture_fields: Set(Some(capture_fields_json)),
            verification: Set(verification_json),
            is_active: Set(is_active),
            display_order: Set(display_order),
            endpoint_usage: Set(endpoint_usage),
            updated_at: Set(now),
            updated_by: Set(updated_by),
            ..Default::default()
        }
        .update(&self.0)
        .await?;

        Ok(into_domain(model))
    }

    async fn delete(&self, id: Uuid) -> anyhow::Result<()> {
        captcha_config::Entity::delete_by_id(id)
            .exec(&self.0)
            .await?;
        Ok(())
    }
}

fn widget_columns(
    widget: Option<&CaptchaWidgetConfig>,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    match widget {
        Some(widget) => (
            empty_to_none(widget.form_field_name.clone()),
            empty_to_none(widget.script_url.clone()),
            empty_to_none(widget.widget_html.clone()),
            widget.script_handler.clone().and_then(empty_to_none),
        ),
        None => (None, None, None, None),
    }
}
