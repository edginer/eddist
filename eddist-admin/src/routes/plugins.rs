use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    repository::plugin_repository::{PluginHook, PluginPermissions, PluginRepository},
    DefaultAppState,
};

#[derive(Debug, Serialize, ToSchema)]
pub struct PluginResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub script: String,
    pub enabled: bool,
    pub hooks: Vec<PluginHook>,
    pub permissions: PluginPermissions,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListPluginsResponse {
    pub plugins: Vec<PluginResponse>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePluginRequest {
    pub name: String,
    pub description: Option<String>,
    pub script: String,
    pub hooks: Vec<PluginHook>,
    pub permissions: PluginPermissions,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdatePluginRequest {
    pub name: String,
    pub description: Option<String>,
    pub script: String,
    pub hooks: Vec<PluginHook>,
    pub permissions: PluginPermissions,
    pub enabled: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TogglePluginRequest {
    pub enabled: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateResponse {
    pub id: String,
}

#[utoipa::path(
    get,
    path = "/plugins",
    tag = "plugins",
    responses(
        (status = 200, description = "List all plugins", body = ListPluginsResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
async fn list_plugins(
    State(state): State<DefaultAppState>,
) -> Result<Json<ListPluginsResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.plugin_repo.list_plugins().await {
        Ok(plugins) => {
            let plugins: Vec<PluginResponse> = plugins
                .into_iter()
                .map(|p| PluginResponse {
                    id: p.id.to_string(),
                    name: p.name,
                    description: p.description,
                    script: p.script,
                    enabled: p.enabled,
                    hooks: p.hooks,
                    permissions: p.permissions,
                    created_at: p.created_at.to_rfc3339(),
                    updated_at: p.updated_at.to_rfc3339(),
                })
                .collect();

            Ok(Json(ListPluginsResponse { plugins }))
        }
        Err(e) => {
            log::error!("Failed to list plugins: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to list plugins".to_string(),
                }),
            ))
        }
    }
}

#[utoipa::path(
    get,
    path = "/plugins/{id}",
    tag = "plugins",
    params(
        ("id" = String, Path, description = "Plugin ID (UUID)")
    ),
    responses(
        (status = 200, description = "Get plugin by ID", body = PluginResponse),
        (status = 404, description = "Plugin not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
async fn get_plugin(
    State(state): State<DefaultAppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<PluginResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.plugin_repo.get_plugin(id).await {
        Ok(Some(p)) => {
            let response = PluginResponse {
                id: p.id.to_string(),
                name: p.name,
                description: p.description,
                script: p.script,
                enabled: p.enabled,
                hooks: p.hooks,
                permissions: p.permissions,
                created_at: p.created_at.to_rfc3339(),
                updated_at: p.updated_at.to_rfc3339(),
            };
            Ok(Json(response))
        }
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Plugin not found".to_string(),
            }),
        )),
        Err(e) => {
            log::error!("Failed to get plugin: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to get plugin".to_string(),
                }),
            ))
        }
    }
}

#[utoipa::path(
    post,
    path = "/plugins",
    tag = "plugins",
    request_body = CreatePluginRequest,
    responses(
        (status = 201, description = "Plugin created", body = CreateResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
async fn create_plugin(
    State(state): State<DefaultAppState>,
    Json(req): Json<CreatePluginRequest>,
) -> Result<(StatusCode, Json<CreateResponse>), (StatusCode, Json<ErrorResponse>)> {
    match state
        .plugin_repo
        .create_plugin(
            req.name,
            req.description,
            req.script,
            req.hooks,
            req.permissions,
        )
        .await
    {
        Ok(id) => Ok((StatusCode::CREATED, Json(CreateResponse { id: id.to_string() }))),
        Err(e) => {
            log::error!("Failed to create plugin: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to create plugin: {}", e),
                }),
            ))
        }
    }
}

#[utoipa::path(
    put,
    path = "/plugins/{id}",
    tag = "plugins",
    params(
        ("id" = String, Path, description = "Plugin ID (UUID)")
    ),
    request_body = UpdatePluginRequest,
    responses(
        (status = 204, description = "Plugin updated"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
async fn update_plugin(
    State(state): State<DefaultAppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdatePluginRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    match state
        .plugin_repo
        .update_plugin(
            id,
            req.name,
            req.description,
            req.script,
            req.hooks,
            req.permissions,
            req.enabled,
        )
        .await
    {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => {
            log::error!("Failed to update plugin: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to update plugin: {}", e),
                }),
            ))
        }
    }
}

#[utoipa::path(
    delete,
    path = "/plugins/{id}",
    tag = "plugins",
    params(
        ("id" = String, Path, description = "Plugin ID (UUID)")
    ),
    responses(
        (status = 204, description = "Plugin deleted"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
async fn delete_plugin(
    State(state): State<DefaultAppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Delete from database
    match state.plugin_repo.delete_plugin(id).await {
        Ok(_) => {
            // Clean up Redis storage for this plugin
            let pattern = format!("plugin:{}:*", id);
            if let Err(e) = cleanup_plugin_redis_storage(state.redis_conn.clone(), &pattern).await {
                log::warn!("Failed to cleanup Redis storage for plugin {}: {}", id, e);
                // Don't fail the request - plugin is already deleted from DB
            }
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            log::error!("Failed to delete plugin: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to delete plugin: {}", e),
                }),
            ))
        }
    }
}

/// Cleanup Redis storage for a deleted plugin
async fn cleanup_plugin_redis_storage(
    mut redis_conn: redis::aio::ConnectionManager,
    pattern: &str,
) -> Result<(), redis::RedisError> {
    use redis::AsyncCommands;

    // Use SCAN to find all keys matching the pattern
    let mut cursor = 0u64;
    let mut total_deleted = 0;

    loop {
        let (new_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(pattern)
            .arg("COUNT")
            .arg(100)
            .query_async(&mut redis_conn)
            .await?;

        // Delete all found keys
        if !keys.is_empty() {
            let deleted: u64 = redis_conn.del(&keys).await?;
            total_deleted += deleted;
        }

        cursor = new_cursor;

        // cursor = 0 means we've scanned all keys
        if cursor == 0 {
            break;
        }
    }

    if total_deleted > 0 {
        log::info!(
            "Cleaned up {} Redis keys matching pattern: {}",
            total_deleted,
            pattern
        );
    }

    Ok(())
}

#[utoipa::path(
    put,
    path = "/plugins/{id}/toggle",
    tag = "plugins",
    params(
        ("id" = String, Path, description = "Plugin ID (UUID)")
    ),
    request_body = TogglePluginRequest,
    responses(
        (status = 204, description = "Plugin toggled"),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
async fn toggle_plugin(
    State(state): State<DefaultAppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<TogglePluginRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    match state.plugin_repo.toggle_plugin(id, req.enabled).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => {
            log::error!("Failed to toggle plugin: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to toggle plugin: {}", e),
                }),
            ))
        }
    }
}

pub fn routes() -> Router<DefaultAppState> {
    Router::new()
        .route("/plugins", get(list_plugins).post(create_plugin))
        .route(
            "/plugins/:id",
            get(get_plugin).put(update_plugin).delete(delete_plugin),
        )
        .route("/plugins/:id/toggle", put(toggle_plugin))
}
