use chrono::Utc;
use eddist_core::{server_settings::KEY_AI_OPENAI_API_KEY, symmetric};
use sqlx::{MySqlPool, query, query_as};
use uuid::Uuid;

use crate::models::server_settings::{ServerSetting, UpsertServerSettingInput};

#[async_trait::async_trait]
pub trait ServerSettingsRepository: Send + Sync {
    async fn get_all(&self) -> anyhow::Result<Vec<ServerSetting>>;
    async fn upsert(&self, input: UpsertServerSettingInput) -> anyhow::Result<ServerSetting>;
}

#[derive(Clone)]
pub struct ServerSettingsRepositoryImpl(MySqlPool);

impl ServerSettingsRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self(pool)
    }
}

#[async_trait::async_trait]
impl ServerSettingsRepository for ServerSettingsRepositoryImpl {
    async fn get_all(&self) -> anyhow::Result<Vec<ServerSetting>> {
        let settings = query_as!(
            ServerSetting,
            r#"
            SELECT
                id AS "id: Uuid",
                setting_key,
                value,
                description,
                created_at,
                updated_at
            FROM server_settings
            ORDER BY setting_key ASC
            "#
        )
        .fetch_all(&self.0)
        .await?;

        let settings = settings
            .into_iter()
            .map(|mut s| {
                if s.setting_key == KEY_AI_OPENAI_API_KEY && !s.value.is_empty() {
                    s.value = "***".to_string();
                }
                s
            })
            .collect();

        Ok(settings)
    }

    async fn upsert(&self, input: UpsertServerSettingInput) -> anyhow::Result<ServerSetting> {
        let id = Uuid::now_v7();
        let now = Utc::now().naive_utc();

        let should_encrypt = input.setting_key == KEY_AI_OPENAI_API_KEY;
        let value = if should_encrypt {
            symmetric::encrypt(&input.value)
        } else {
            input.value.clone()
        };

        query!(
            r#"
            INSERT INTO server_settings (id, setting_key, value, description, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
                value = VALUES(value),
                description = VALUES(description),
                updated_at = VALUES(updated_at)
            "#,
            id,
            input.setting_key,
            value,
            input.description,
            now,
            now
        )
        .execute(&self.0)
        .await?;

        let setting = query_as!(
            ServerSetting,
            r#"
            SELECT
                id AS "id: Uuid",
                setting_key,
                value,
                description,
                created_at,
                updated_at
            FROM server_settings
            WHERE setting_key = ?
            "#,
            input.setting_key
        )
        .fetch_one(&self.0)
        .await?;

        if setting.setting_key == KEY_AI_OPENAI_API_KEY && !setting.value.is_empty() {
            return Ok(ServerSetting {
                value: "***".to_string(),
                ..setting
            });
        }

        Ok(setting)
    }
}
