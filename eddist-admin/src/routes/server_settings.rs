use axum::{
    extract::State,
    routing::{get, put},
    Json, Router,
};

use crate::{
    error::ApiError,
    models::server_settings::{ServerSetting, UpsertServerSettingInput},
    AppState,
};

/// Known setting keys and their descriptions.
/// Only these keys are allowed to be created.
const KNOWN_SETTINGS: &[(&str, &str)] = &[(
    "require_user_registration",
    "Require users to link an external IdP account before posting (true/false)",
)];

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/server-settings", get(list_server_settings))
        .route("/server-settings", put(upsert_server_setting))
}

#[utoipa::path(
    get,
    path = "/server-settings/",
    tag = "server_settings",
    responses(
        (status = 200, description = "List all server settings successfully", body = Vec<ServerSetting>),
    )
)]
pub async fn list_server_settings(
    State(state): State<AppState>,
) -> Result<Json<Vec<ServerSetting>>, ApiError> {
    let settings = state.server_settings_repo.get_all().await?;
    Ok(Json(settings))
}

#[utoipa::path(
    put,
    path = "/server-settings/",
    tag = "server_settings",
    request_body = UpsertServerSettingInput,
    responses(
        (status = 200, description = "Server setting upserted successfully", body = ServerSetting),
        (status = 400, description = "Invalid input"),
    )
)]
pub async fn upsert_server_setting(
    State(state): State<AppState>,
    Json(input): Json<UpsertServerSettingInput>,
) -> Result<Json<ServerSetting>, ApiError> {
    if !KNOWN_SETTINGS
        .iter()
        .any(|(key, _)| *key == input.setting_key)
    {
        return Err(ApiError::bad_request(format!(
            "Unknown setting key: '{}'. Known keys: {}",
            input.setting_key,
            KNOWN_SETTINGS
                .iter()
                .map(|(k, _)| *k)
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }

    let setting = state
        .server_settings_repo
        .upsert(input)
        .await
        .map_err(|e| ApiError::bad_request(format!("Failed to upsert server setting: {e}")))?;
    Ok(Json(setting))
}
