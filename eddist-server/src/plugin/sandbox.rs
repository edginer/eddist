use crate::plugin::{
    api::register_eddist_api, hooks::HookPoint, http::PluginHttpClient, model::Plugin,
    storage::PluginStorage,
};
use anyhow::{anyhow, Result};
use mlua::{Lua, LuaOptions, LuaSerdeExt, StdLib, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::timeout;

const MAX_EXECUTION_TIME_WITH_HTTP: Duration = Duration::from_secs(1);
const MAX_EXECUTION_TIME_NO_HTTP: Duration = Duration::from_millis(500);
const MAX_MEMORY: usize = 10 * 1024 * 1024; // 10MB

pub struct PluginSandbox {
    plugin: Plugin,
    lua: Lua,
    execution_timeout: Duration,
}

impl PluginSandbox {
    pub fn new(plugin: Plugin, redis: redis::aio::ConnectionManager) -> Result<Self> {
        // Create Lua instance with limited standard library
        let lua = Lua::new_with(
            StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::UTF8,
            LuaOptions::default(),
        )?;

        // Set memory limit
        lua.set_memory_limit(MAX_MEMORY)?;

        // Remove dangerous globals explicitly
        lua.globals().set("io", mlua::Nil)?;
        lua.globals().set("os", mlua::Nil)?;
        lua.globals().set("require", mlua::Nil)?;
        lua.globals().set("dofile", mlua::Nil)?;
        lua.globals().set("load", mlua::Nil)?;
        lua.globals().set("loadfile", mlua::Nil)?;
        lua.globals().set("loadstring", mlua::Nil)?;
        lua.globals().set("module", mlua::Nil)?;
        lua.globals().set("package", mlua::Nil)?;

        // Set up storage (only if allowed)
        let storage = if plugin.permissions.allow_storage {
            Some(Arc::new(Mutex::new(PluginStorage::new(redis, plugin.id))))
        } else {
            None
        };

        // Set up HTTP client (only if allowed and whitelist exists)
        let http_client = if plugin.permissions.allow_http && !plugin.permissions.http_whitelist.is_empty() {
            Some(Arc::new(PluginHttpClient::new(plugin.permissions.http_whitelist.clone())))
        } else {
            None
        };

        // Register eddist API
        register_eddist_api(&lua, storage, http_client)?;

        // Load the plugin script
        lua.load(&plugin.script).exec()?;

        // Determine execution timeout based on permissions
        let execution_timeout = if plugin.permissions.allow_http {
            MAX_EXECUTION_TIME_WITH_HTTP
        } else {
            MAX_EXECUTION_TIME_NO_HTTP
        };

        Ok(Self { plugin, lua, execution_timeout })
    }

    pub async fn execute_hook(
        &self,
        hook: HookPoint,
        data: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let function_name = hook.function_name();

        // Check if the function exists
        let hook_fn: mlua::Function = self.lua.globals().get(function_name)?;

        // Convert JSON to Lua value using serde
        let lua_value: Value = self.lua.to_value(&data)?;

        // Execute with timeout (dynamic based on permissions)
        let result = timeout(self.execution_timeout, async {
            hook_fn.call_async::<Value>(lua_value).await
        })
        .await
        .map_err(|_| anyhow!("Hook execution timed out after {:?}", self.execution_timeout))??;

        // Convert Lua value back to JSON using serde
        let result_json: serde_json::Value = self.lua.from_value(result)?;

        Ok(result_json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::model::PluginHook;

    #[tokio::test]
    async fn test_sandbox_basic_execution() {
        use crate::plugin::model::PluginPermissions;

        let plugin = Plugin {
            id: uuid::Uuid::now_v7(),
            name: "test".to_string(),
            description: None,
            script: r#"
                function before_post_response(response)
                    response.content = "modified"
                    return response
                end
            "#
            .to_string(),
            enabled: true,
            hooks: vec![PluginHook::BeforePostResponse],
            permissions: PluginPermissions::default(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let redis_client = redis::Client::open("redis://127.0.0.1/").unwrap();
        let redis = redis::aio::ConnectionManager::new(redis_client)
            .await
            .unwrap();

        let sandbox = PluginSandbox::new(plugin, redis).unwrap();

        let data = serde_json::json!({
            "content": "original",
            "author_id": "test123"
        });

        let result = sandbox
            .execute_hook(HookPoint::BeforePostResponse, data)
            .await
            .unwrap();

        assert_eq!(result["content"], "modified");
    }
}
