use crate::plugin::{hooks::HookPoint, model::Plugin, sandbox::PluginSandbox};
use anyhow::Result;
use sqlx::MySqlPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct PluginManager {
    db: MySqlPool,
    redis: redis::aio::ConnectionManager,
}

impl PluginManager {
    pub fn new(db: MySqlPool, redis: redis::aio::ConnectionManager) -> Self {
        Self { db, redis }
    }

    /// Get all enabled plugins for a specific hook point
    pub async fn get_plugins_for_hook(&self, hook: HookPoint) -> Result<Vec<Plugin>> {
        let hook_enum: crate::plugin::model::PluginHook = hook.into();
        let plugins = sqlx::query_as::<_, Plugin>(
            r#"
            SELECT * FROM plugins
            WHERE enabled = TRUE
            AND JSON_CONTAINS(hooks, ?)
            ORDER BY id ASC
            "#,
        )
        .bind(serde_json::to_string(&hook_enum)?)
        .fetch_all(&self.db)
        .await?;

        Ok(plugins)
    }

    /// Execute a hook for all enabled plugins
    pub async fn execute_hook(
        &self,
        hook: HookPoint,
        mut data: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let plugins = self.get_plugins_for_hook(hook).await?;

        for plugin in plugins {
            let sandbox = PluginSandbox::new(plugin.clone(), self.redis.clone())?;

            match sandbox.execute_hook(hook, data.clone()).await {
                Ok(result) => {
                    data = result;
                }
                Err(e) => {
                    log::error!("Plugin '{}' failed at hook {:?}: {}", plugin.name, hook, e);
                    // Continue with next plugin
                }
            }
        }

        Ok(data)
    }

    /// Get plugin by ID
    pub async fn get_plugin(&self, id: Uuid) -> Result<Option<Plugin>> {
        let plugin = sqlx::query_as::<_, Plugin>("SELECT * FROM plugins WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.db)
            .await?;

        Ok(plugin)
    }

    /// Get all plugins
    pub async fn list_plugins(&self) -> Result<Vec<Plugin>> {
        let plugins = sqlx::query_as::<_, Plugin>("SELECT * FROM plugins ORDER BY id ASC")
            .fetch_all(&self.db)
            .await?;

        Ok(plugins)
    }

    /// Create a new plugin
    pub async fn create_plugin(
        &self,
        name: String,
        description: Option<String>,
        script: String,
        hooks: Vec<crate::plugin::model::PluginHook>,
        permissions: crate::plugin::model::PluginPermissions,
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
        .execute(&self.db)
        .await?;

        Ok(id)
    }

    /// Update an existing plugin
    pub async fn update_plugin(
        &self,
        id: Uuid,
        name: String,
        description: Option<String>,
        script: String,
        hooks: Vec<crate::plugin::model::PluginHook>,
        permissions: crate::plugin::model::PluginPermissions,
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
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Delete a plugin
    pub async fn delete_plugin(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM plugins WHERE id = ?")
            .bind(id)
            .execute(&self.db)
            .await?;

        // TODO: Clean up Redis storage for this plugin

        Ok(())
    }

    /// Toggle plugin enabled status
    pub async fn toggle_plugin(&self, id: Uuid, enabled: bool) -> Result<()> {
        let now = chrono::Utc::now();

        sqlx::query("UPDATE plugins SET enabled = ?, updated_at = ? WHERE id = ?")
            .bind(enabled)
            .bind(now)
            .bind(id)
            .execute(&self.db)
            .await?;

        Ok(())
    }
}
