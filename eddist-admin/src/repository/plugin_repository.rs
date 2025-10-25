use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, MySqlPool};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PluginHook {
    BeforePostThread,
    AfterPostThread,
    BeforePostResponse,
    AfterPostResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct PluginPermissions {
    #[serde(default)]
    pub allow_http: bool,
    #[serde(default)]
    pub http_whitelist: Vec<HttpWhitelistEntry>,
    #[serde(default)]
    pub allow_storage: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HttpWhitelistEntry {
    pub url_pattern: String,
    pub methods: Vec<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct Plugin {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub script: String,
    pub enabled: bool,
    #[sqlx(json)]
    pub hooks: Vec<PluginHook>,
    #[sqlx(json)]
    pub permissions: PluginPermissions,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[async_trait]
pub trait PluginRepository: Send + Sync + 'static {
    async fn get_plugin(&self, id: Uuid) -> Result<Option<Plugin>>;
    async fn list_plugins(&self) -> Result<Vec<Plugin>>;
    async fn create_plugin(
        &self,
        name: String,
        description: Option<String>,
        script: String,
        hooks: Vec<PluginHook>,
        permissions: PluginPermissions,
    ) -> Result<Uuid>;
    async fn update_plugin(
        &self,
        id: Uuid,
        name: String,
        description: Option<String>,
        script: String,
        hooks: Vec<PluginHook>,
        permissions: PluginPermissions,
        enabled: bool,
    ) -> Result<()>;
    async fn delete_plugin(&self, id: Uuid) -> Result<()>;
    async fn toggle_plugin(&self, id: Uuid, enabled: bool) -> Result<()>;
}

#[derive(Clone)]
pub struct PluginRepositoryImpl {
    pool: MySqlPool,
}

impl PluginRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PluginRepository for PluginRepositoryImpl {
    async fn get_plugin(&self, id: Uuid) -> Result<Option<Plugin>> {
        let plugin = sqlx::query_as::<_, Plugin>("SELECT * FROM plugins WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(plugin)
    }

    async fn list_plugins(&self) -> Result<Vec<Plugin>> {
        let plugins = sqlx::query_as::<_, Plugin>("SELECT * FROM plugins ORDER BY id ASC")
            .fetch_all(&self.pool)
            .await?;
        Ok(plugins)
    }

    async fn create_plugin(
        &self,
        name: String,
        description: Option<String>,
        script: String,
        hooks: Vec<PluginHook>,
        permissions: PluginPermissions,
    ) -> Result<Uuid> {
        let id = Uuid::now_v7();
        let now = chrono::Utc::now();

        sqlx::query(
            r#"
            INSERT INTO plugins (id, name, description, script, hooks, permissions, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(&name)
        .bind(&description)
        .bind(&script)
        .bind(serde_json::to_string(&hooks)?)
        .bind(serde_json::to_value(&permissions)?)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    async fn update_plugin(
        &self,
        id: Uuid,
        name: String,
        description: Option<String>,
        script: String,
        hooks: Vec<PluginHook>,
        permissions: PluginPermissions,
        enabled: bool,
    ) -> Result<()> {
        let now = chrono::Utc::now();

        sqlx::query(
            r#"
            UPDATE plugins
            SET name = ?, description = ?, script = ?, hooks = ?, permissions = ?, enabled = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&name)
        .bind(&description)
        .bind(&script)
        .bind(serde_json::to_string(&hooks)?)
        .bind(serde_json::to_value(&permissions)?)
        .bind(enabled)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete_plugin(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM plugins WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn toggle_plugin(&self, id: Uuid, enabled: bool) -> Result<()> {
        let now = chrono::Utc::now();

        sqlx::query("UPDATE plugins SET enabled = ?, updated_at = ? WHERE id = ?")
            .bind(enabled)
            .bind(now)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
