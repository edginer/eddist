use chrono::Utc;
use sqlx::{query, query_as, MySqlPool};
use uuid::Uuid;

use crate::models::{
    CaptchaConfig, CaptchaVerificationConfig, CaptchaWidgetConfig, CreateCaptchaConfigInput,
    UpdateCaptchaConfigInput,
};

/// Convert empty strings to None (for storing NULL in database)
fn empty_to_none(s: String) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct CaptchaConfigRow {
    id: Uuid,
    name: String,
    provider: String,
    site_key: String,
    secret: String,
    base_url: Option<String>,
    widget_form_field_name: Option<String>,
    widget_script_url: Option<String>,
    widget_html: Option<String>,
    widget_script_handler: Option<String>,
    capture_fields: Option<serde_json::Value>,
    verification: Option<serde_json::Value>,
    is_active: bool,
    display_order: i32,
    created_at: chrono::NaiveDateTime,
    updated_at: chrono::NaiveDateTime,
    updated_by: Option<String>,
}

impl From<CaptchaConfigRow> for CaptchaConfig {
    fn from(row: CaptchaConfigRow) -> Self {
        let capture_fields: Vec<String> = row
            .capture_fields
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        let verification: Option<CaptchaVerificationConfig> = row
            .verification
            .and_then(|v| serde_json::from_value(v).ok());

        // Widget is only present if all required fields are set
        let widget = match (
            row.widget_form_field_name,
            row.widget_script_url,
            row.widget_html,
        ) {
            (Some(form_field_name), Some(script_url), Some(widget_html)) => {
                Some(CaptchaWidgetConfig {
                    form_field_name,
                    script_url,
                    widget_html,
                    script_handler: row.widget_script_handler,
                })
            }
            _ => None,
        };

        CaptchaConfig {
            id: row.id,
            name: row.name,
            provider: row.provider,
            site_key: row.site_key,
            secret: row.secret,
            base_url: row.base_url,
            widget,
            capture_fields,
            verification,
            is_active: row.is_active,
            display_order: row.display_order,
            created_at: row.created_at,
            updated_at: row.updated_at,
            updated_by: row.updated_by,
        }
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
pub struct CaptchaConfigRepositoryImpl(MySqlPool);

impl CaptchaConfigRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self(pool)
    }
}

#[async_trait::async_trait]
impl CaptchaConfigRepository for CaptchaConfigRepositoryImpl {
    async fn get_all(&self) -> anyhow::Result<Vec<CaptchaConfig>> {
        let rows = query_as!(
            CaptchaConfigRow,
            r#"
            SELECT
                id AS "id: Uuid",
                name,
                provider,
                site_key,
                secret,
                base_url,
                widget_form_field_name,
                widget_script_url,
                widget_html,
                widget_script_handler,
                capture_fields AS "capture_fields: serde_json::Value",
                verification AS "verification: serde_json::Value",
                is_active AS "is_active: bool",
                display_order,
                created_at,
                updated_at,
                updated_by
            FROM captcha_configs
            ORDER BY display_order ASC, created_at ASC
            "#
        )
        .fetch_all(&self.0)
        .await?;

        Ok(rows.into_iter().map(CaptchaConfig::from).collect())
    }

    async fn get_active(&self) -> anyhow::Result<Vec<CaptchaConfig>> {
        let rows = query_as!(
            CaptchaConfigRow,
            r#"
            SELECT
                id AS "id: Uuid",
                name,
                provider,
                site_key,
                secret,
                base_url,
                widget_form_field_name,
                widget_script_url,
                widget_html,
                widget_script_handler,
                capture_fields AS "capture_fields: serde_json::Value",
                verification AS "verification: serde_json::Value",
                is_active AS "is_active: bool",
                display_order,
                created_at,
                updated_at,
                updated_by
            FROM captcha_configs
            WHERE is_active = 1
            ORDER BY display_order ASC, created_at ASC
            "#
        )
        .fetch_all(&self.0)
        .await?;

        Ok(rows.into_iter().map(CaptchaConfig::from).collect())
    }

    async fn get_by_id(&self, id: Uuid) -> anyhow::Result<Option<CaptchaConfig>> {
        let row = query_as!(
            CaptchaConfigRow,
            r#"
            SELECT
                id AS "id: Uuid",
                name,
                provider,
                site_key,
                secret,
                base_url,
                widget_form_field_name,
                widget_script_url,
                widget_html,
                widget_script_handler,
                capture_fields AS "capture_fields: serde_json::Value",
                verification AS "verification: serde_json::Value",
                is_active AS "is_active: bool",
                display_order,
                created_at,
                updated_at,
                updated_by
            FROM captcha_configs
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(&self.0)
        .await?;

        Ok(row.map(CaptchaConfig::from))
    }

    async fn create(
        &self,
        input: CreateCaptchaConfigInput,
        updated_by: Option<String>,
    ) -> anyhow::Result<CaptchaConfig> {
        let id = Uuid::now_v7();
        let now = Utc::now().naive_utc();

        let capture_fields_json = serde_json::to_value(&input.capture_fields)?;
        let verification_json = input
            .verification
            .as_ref()
            .map(|v| serde_json::to_value(v))
            .transpose()?;

        let (form_field_name, script_url, widget_html, script_handler) =
            match input.widget.as_ref() {
                Some(w) => (
                    empty_to_none(w.form_field_name.clone()),
                    empty_to_none(w.script_url.clone()),
                    empty_to_none(w.widget_html.clone()),
                    w.script_handler.clone().and_then(empty_to_none),
                ),
                None => (None, None, None, None),
            };

        query!(
            r#"
            INSERT INTO captcha_configs (
                id, name, provider, site_key, secret, base_url,
                widget_form_field_name, widget_script_url, widget_html, widget_script_handler,
                capture_fields, verification,
                is_active, display_order, created_at, updated_at, updated_by
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            id,
            input.name,
            input.provider,
            input.site_key,
            input.secret,
            input.base_url,
            form_field_name,
            script_url,
            widget_html,
            script_handler,
            capture_fields_json,
            verification_json,
            input.is_active,
            input.display_order,
            now,
            now,
            updated_by
        )
        .execute(&self.0)
        .await?;

        Ok(CaptchaConfig {
            id,
            name: input.name,
            provider: input.provider,
            site_key: input.site_key,
            secret: input.secret,
            base_url: input.base_url,
            widget: input.widget,
            capture_fields: input.capture_fields,
            verification: input.verification,
            is_active: input.is_active,
            display_order: input.display_order,
            created_at: now,
            updated_at: now,
            updated_by,
        })
    }

    async fn update(
        &self,
        id: Uuid,
        input: UpdateCaptchaConfigInput,
        updated_by: Option<String>,
    ) -> anyhow::Result<CaptchaConfig> {
        let now = Utc::now().naive_utc();

        let current = self
            .get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Captcha config not found"))?;

        let name = input.name.unwrap_or(current.name);
        let provider = input.provider.unwrap_or(current.provider);
        let site_key = input.site_key.unwrap_or(current.site_key);
        let secret = input.secret.unwrap_or(current.secret);
        let base_url = input.base_url.or(current.base_url);
        let widget = input.widget.or(current.widget);
        let capture_fields = input.capture_fields.unwrap_or(current.capture_fields);
        let verification = input.verification.or(current.verification);
        let is_active = input.is_active.unwrap_or(current.is_active);
        let display_order = input.display_order.unwrap_or(current.display_order);

        let capture_fields_json = serde_json::to_value(&capture_fields)?;
        let verification_json = verification
            .as_ref()
            .map(|v| serde_json::to_value(v))
            .transpose()?;

        let (form_field_name, script_url, widget_html_val, script_handler) = match widget.as_ref() {
            Some(w) => (
                empty_to_none(w.form_field_name.clone()),
                empty_to_none(w.script_url.clone()),
                empty_to_none(w.widget_html.clone()),
                w.script_handler.clone().and_then(empty_to_none),
            ),
            None => (None, None, None, None),
        };

        query!(
            r#"
            UPDATE captcha_configs
            SET name = ?, provider = ?, site_key = ?, secret = ?, base_url = ?,
                widget_form_field_name = ?, widget_script_url = ?, widget_html = ?, widget_script_handler = ?,
                capture_fields = ?, verification = ?,
                is_active = ?, display_order = ?, updated_at = ?, updated_by = ?
            WHERE id = ?
            "#,
            name,
            provider,
            site_key,
            secret,
            base_url,
            form_field_name,
            script_url,
            widget_html_val,
            script_handler,
            capture_fields_json,
            verification_json,
            is_active,
            display_order,
            now,
            updated_by,
            id
        )
        .execute(&self.0)
        .await?;

        Ok(CaptchaConfig {
            id,
            name,
            provider,
            site_key,
            secret,
            base_url,
            widget,
            capture_fields,
            verification,
            is_active,
            display_order,
            created_at: current.created_at,
            updated_at: now,
            updated_by,
        })
    }

    async fn delete(&self, id: Uuid) -> anyhow::Result<()> {
        query!(
            r#"
            DELETE FROM captcha_configs
            WHERE id = ?
            "#,
            id
        )
        .execute(&self.0)
        .await?;

        Ok(())
    }
}
