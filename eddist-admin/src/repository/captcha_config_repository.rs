use chrono::Utc;
#[cfg(not(feature = "backend-postgres"))]
use sqlx::{MySqlPool, query, query_as};
use uuid::Uuid;

#[cfg(feature = "backend-postgres")]
#[derive(Debug, Clone, sqlx::FromRow)]
struct CaptchaConfigRowPg {
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
    endpoint_usage: String,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
    updated_by: Option<String>,
}

#[cfg(feature = "backend-postgres")]
impl From<CaptchaConfigRowPg> for CaptchaConfig {
    fn from(row: CaptchaConfigRowPg) -> Self {
        let capture_fields: Vec<String> = row
            .capture_fields
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        let verification: Option<CaptchaVerificationConfig> = row
            .verification
            .and_then(|v| serde_json::from_value(v).ok());

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
            endpoint_usage: row.endpoint_usage,
            created_at: row.created_at.naive_utc(),
            updated_at: row.updated_at.naive_utc(),
            updated_by: row.updated_by,
        }
    }
}

use crate::models::{
    CaptchaConfig, CaptchaVerificationConfig, CaptchaWidgetConfig, CreateCaptchaConfigInput,
    UpdateCaptchaConfigInput,
};

/// Convert empty strings to None (for storing NULL in database)
fn empty_to_none(s: String) -> Option<String> {
    if s.is_empty() { None } else { Some(s) }
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
    endpoint_usage: String,
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
            endpoint_usage: row.endpoint_usage,
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

#[cfg(not(feature = "backend-postgres"))]
#[derive(Clone)]
pub struct CaptchaConfigRepositoryImpl(MySqlPool);

#[cfg(not(feature = "backend-postgres"))]
impl CaptchaConfigRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self(pool)
    }
}

#[cfg(not(feature = "backend-postgres"))]
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
                endpoint_usage,
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
                endpoint_usage,
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
                endpoint_usage,
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
            .map(serde_json::to_value)
            .transpose()?;

        let (form_field_name, script_url, widget_html, script_handler) = match input.widget.as_ref()
        {
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
                is_active, display_order, endpoint_usage, created_at, updated_at, updated_by
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
            input.endpoint_usage,
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
            endpoint_usage: input.endpoint_usage,
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
        let secret = input
            .secret
            .filter(|s| !s.is_empty())
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
                is_active = ?, display_order = ?, endpoint_usage = ?, updated_at = ?, updated_by = ?
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
            endpoint_usage,
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
            endpoint_usage,
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

#[cfg(feature = "backend-postgres")]
#[derive(Clone)]
pub struct CaptchaConfigRepositoryPgImpl(sqlx::PgPool);

#[cfg(feature = "backend-postgres")]
impl CaptchaConfigRepositoryPgImpl {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self(pool)
    }

    async fn get_by_id_pg(&self, id: Uuid) -> anyhow::Result<Option<CaptchaConfig>> {
        let row = sqlx::query_as::<_, CaptchaConfigRowPg>(
            r#"
            SELECT
                id, name, provider, site_key, secret, base_url,
                widget_form_field_name, widget_script_url, widget_html, widget_script_handler,
                capture_fields, verification, is_active, display_order, endpoint_usage,
                created_at, updated_at, updated_by
            FROM captcha_configs
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.0)
        .await?;
        Ok(row.map(CaptchaConfig::from))
    }
}

#[cfg(feature = "backend-postgres")]
#[async_trait::async_trait]
impl CaptchaConfigRepository for CaptchaConfigRepositoryPgImpl {
    async fn get_all(&self) -> anyhow::Result<Vec<CaptchaConfig>> {
        let rows = sqlx::query_as::<_, CaptchaConfigRowPg>(
            r#"
            SELECT
                id, name, provider, site_key, secret, base_url,
                widget_form_field_name, widget_script_url, widget_html, widget_script_handler,
                capture_fields, verification, is_active, display_order, endpoint_usage,
                created_at, updated_at, updated_by
            FROM captcha_configs
            ORDER BY display_order ASC, created_at ASC
            "#,
        )
        .fetch_all(&self.0)
        .await?;
        Ok(rows.into_iter().map(CaptchaConfig::from).collect())
    }

    async fn get_active(&self) -> anyhow::Result<Vec<CaptchaConfig>> {
        let rows = sqlx::query_as::<_, CaptchaConfigRowPg>(
            r#"
            SELECT
                id, name, provider, site_key, secret, base_url,
                widget_form_field_name, widget_script_url, widget_html, widget_script_handler,
                capture_fields, verification, is_active, display_order, endpoint_usage,
                created_at, updated_at, updated_by
            FROM captcha_configs
            WHERE is_active = TRUE
            ORDER BY display_order ASC, created_at ASC
            "#,
        )
        .fetch_all(&self.0)
        .await?;
        Ok(rows.into_iter().map(CaptchaConfig::from).collect())
    }

    async fn get_by_id(&self, id: Uuid) -> anyhow::Result<Option<CaptchaConfig>> {
        self.get_by_id_pg(id).await
    }

    async fn create(
        &self,
        input: CreateCaptchaConfigInput,
        updated_by: Option<String>,
    ) -> anyhow::Result<CaptchaConfig> {
        let id = Uuid::now_v7();
        let now = Utc::now();

        let capture_fields_json = serde_json::to_value(&input.capture_fields)?;
        let verification_json = input
            .verification
            .as_ref()
            .map(serde_json::to_value)
            .transpose()?;

        let (form_field_name, script_url, widget_html, script_handler) = match input.widget.as_ref()
        {
            Some(w) => (
                empty_to_none(w.form_field_name.clone()),
                empty_to_none(w.script_url.clone()),
                empty_to_none(w.widget_html.clone()),
                w.script_handler.clone().and_then(empty_to_none),
            ),
            None => (None, None, None, None),
        };

        sqlx::query(
            r#"
            INSERT INTO captcha_configs (
                id, name, provider, site_key, secret, base_url,
                widget_form_field_name, widget_script_url, widget_html, widget_script_handler,
                capture_fields, verification,
                is_active, display_order, endpoint_usage, created_at, updated_at, updated_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            "#,
        )
        .bind(id)
        .bind(&input.name)
        .bind(&input.provider)
        .bind(&input.site_key)
        .bind(&input.secret)
        .bind(&input.base_url)
        .bind(&form_field_name)
        .bind(&script_url)
        .bind(&widget_html)
        .bind(&script_handler)
        .bind(&capture_fields_json)
        .bind(&verification_json)
        .bind(input.is_active)
        .bind(input.display_order)
        .bind(&input.endpoint_usage)
        .bind(now)
        .bind(now)
        .bind(&updated_by)
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
            endpoint_usage: input.endpoint_usage,
            created_at: now.naive_utc(),
            updated_at: now.naive_utc(),
            updated_by,
        })
    }

    async fn update(
        &self,
        id: Uuid,
        input: UpdateCaptchaConfigInput,
        updated_by: Option<String>,
    ) -> anyhow::Result<CaptchaConfig> {
        let now = Utc::now();

        let current = self
            .get_by_id_pg(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Captcha config not found"))?;

        let name = input.name.unwrap_or(current.name);
        let provider = input.provider.unwrap_or(current.provider);
        let site_key = input.site_key.unwrap_or(current.site_key);
        let secret = input
            .secret
            .filter(|s| !s.is_empty())
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

        let (form_field_name, script_url, widget_html_val, script_handler) = match widget.as_ref() {
            Some(w) => (
                empty_to_none(w.form_field_name.clone()),
                empty_to_none(w.script_url.clone()),
                empty_to_none(w.widget_html.clone()),
                w.script_handler.clone().and_then(empty_to_none),
            ),
            None => (None, None, None, None),
        };

        sqlx::query(
            r#"
            UPDATE captcha_configs
            SET name = $1, provider = $2, site_key = $3, secret = $4, base_url = $5,
                widget_form_field_name = $6, widget_script_url = $7, widget_html = $8, widget_script_handler = $9,
                capture_fields = $10, verification = $11,
                is_active = $12, display_order = $13, endpoint_usage = $14, updated_at = $15, updated_by = $16
            WHERE id = $17
            "#,
        )
        .bind(&name)
        .bind(&provider)
        .bind(&site_key)
        .bind(&secret)
        .bind(&base_url)
        .bind(&form_field_name)
        .bind(&script_url)
        .bind(&widget_html_val)
        .bind(&script_handler)
        .bind(&capture_fields_json)
        .bind(&verification_json)
        .bind(is_active)
        .bind(display_order)
        .bind(&endpoint_usage)
        .bind(now)
        .bind(&updated_by)
        .bind(id)
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
            endpoint_usage,
            created_at: current.created_at,
            updated_at: now.naive_utc(),
            updated_by,
        })
    }

    async fn delete(&self, id: Uuid) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM captcha_configs WHERE id = $1")
            .bind(id)
            .execute(&self.0)
            .await?;
        Ok(())
    }
}
