use crate::plugin::{http::PluginHttpClient, storage::PluginStorage};
use mlua::{Lua, Result as LuaResult, Table};
use std::sync::Arc;
use tokio::sync::Mutex;

pub fn register_eddist_api(
    lua: &Lua,
    storage: Option<Arc<Mutex<PluginStorage>>>,
    http_client: Option<Arc<PluginHttpClient>>,
) -> LuaResult<()> {
    let eddist = lua.create_table()?;

    // Content manipulation functions
    register_content_api(lua, &eddist)?;

    // Storage API (only if allowed)
    if let Some(storage) = storage {
        register_storage_api(lua, &eddist, storage)?;
    }

    // HTTP API (only if allowed)
    if let Some(client) = http_client {
        register_http_api(lua, &eddist, client)?;
    }

    // Logging API
    register_log_api(lua, &eddist)?;

    lua.globals().set("eddist", eddist)?;

    Ok(())
}

fn register_content_api(lua: &Lua, eddist: &Table) -> LuaResult<()> {
    // eddist.get_content(post) -> string
    let get_content = lua.create_async_function(|_, post: Table| async move {
        post.get::<String>("content")
    })?;
    eddist.set("get_content", get_content)?;

    // eddist.set_content(post, new_content) -> bool
    let set_content = lua.create_async_function(|_, (post, content): (Table, String)| async move {
        post.set("content", content)?;
        Ok(true)
    })?;
    eddist.set("set_content", set_content)?;

    // eddist.get_author_id(post) -> string
    let get_author_id = lua.create_async_function(|_, post: Table| async move {
        post.get::<String>("author_id")
    })?;
    eddist.set("get_author_id", get_author_id)?;

    Ok(())
}

fn register_storage_api(
    lua: &Lua,
    eddist: &Table,
    storage: Arc<Mutex<PluginStorage>>,
) -> LuaResult<()> {
    let storage_table = lua.create_table()?;

    // eddist.storage.get(key) -> string|nil
    let storage_get = storage.clone();
    let get = lua.create_async_function(move |_, key: String| {
        let storage = storage_get.clone();
        async move {
            let mut storage = storage.lock().await;
            storage
                .get(&key)
                .await
                .map_err(|e| mlua::Error::external(e))
        }
    })?;
    storage_table.set("get", get)?;

    // eddist.storage.set(key, value, ttl?) -> bool
    let storage_set = storage.clone();
    let set = lua.create_async_function(move |_, (key, value, ttl): (String, String, Option<i64>)| {
        let storage = storage_set.clone();
        async move {
            let mut storage = storage.lock().await;
            storage
                .set(&key, &value, ttl)
                .await
                .map_err(|e| mlua::Error::external(e))
        }
    })?;
    storage_table.set("set", set)?;

    // eddist.storage.delete(key) -> bool
    let storage_delete = storage.clone();
    let delete = lua.create_async_function(move |_, key: String| {
        let storage = storage_delete.clone();
        async move {
            let mut storage = storage.lock().await;
            storage
                .delete(&key)
                .await
                .map_err(|e| mlua::Error::external(e))
        }
    })?;
    storage_table.set("delete", delete)?;

    // eddist.storage.exists(key) -> bool
    let storage_exists = storage;
    let exists = lua.create_async_function(move |_, key: String| {
        let storage = storage_exists.clone();
        async move {
            let mut storage = storage.lock().await;
            storage
                .exists(&key)
                .await
                .map_err(|e| mlua::Error::external(e))
        }
    })?;
    storage_table.set("exists", exists)?;

    eddist.set("storage", storage_table)?;

    Ok(())
}

fn register_http_api(
    lua: &Lua,
    eddist: &Table,
    http_client: Arc<PluginHttpClient>,
) -> LuaResult<()> {
    let http_table = lua.create_table()?;

    // eddist.http.get(url) -> {status, body, headers}
    let client_get = http_client.clone();
    let get = lua.create_async_function(move |lua, url: String| {
        let client = client_get.clone();
        async move {
            let response = client
                .get(&url)
                .await
                .map_err(|e| mlua::Error::external(e))?;

            let table = lua.create_table()?;
            table.set("status", response.status)?;
            table.set("body", response.body)?;

            let headers = lua.create_table()?;
            for (k, v) in response.headers {
                headers.set(k, v)?;
            }
            table.set("headers", headers)?;

            Ok(table)
        }
    })?;
    http_table.set("get", get)?;

    // eddist.http.post(url, body) -> {status, body, headers}
    let client_post = http_client;
    let post = lua.create_async_function(move |lua, (url, body): (String, String)| {
        let client = client_post.clone();
        async move {
            let response = client
                .post(&url, &body)
                .await
                .map_err(|e| mlua::Error::external(e))?;

            let table = lua.create_table()?;
            table.set("status", response.status)?;
            table.set("body", response.body)?;

            let headers = lua.create_table()?;
            for (k, v) in response.headers {
                headers.set(k, v)?;
            }
            table.set("headers", headers)?;

            Ok(table)
        }
    })?;
    http_table.set("post", post)?;

    eddist.set("http", http_table)?;

    Ok(())
}

fn register_log_api(lua: &Lua, eddist: &Table) -> LuaResult<()> {
    // eddist.log(level, message)
    let log_fn = lua.create_function(|_, (level, message): (String, String)| {
        match level.as_str() {
            "error" => log::error!("[Plugin] {}", message),
            "warn" => log::warn!("[Plugin] {}", message),
            "info" => log::info!("[Plugin] {}", message),
            "debug" => log::debug!("[Plugin] {}", message),
            _ => log::info!("[Plugin] {}", message),
        }
        Ok(())
    })?;
    eddist.set("log", log_fn)?;

    Ok(())
}
