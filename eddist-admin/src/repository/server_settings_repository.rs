use crate::entity::server_settings;
use eddist_core::{server_settings::KEY_AI_OPENAI_API_KEY, symmetric};
use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
};
use uuid::Uuid;

use crate::models::server_settings::{ServerSetting, UpsertServerSettingInput};

fn upsert_on_conflict() -> OnConflict {
    OnConflict::column(server_settings::Column::SettingKey)
        .update_columns([
            server_settings::Column::Value,
            server_settings::Column::Description,
            server_settings::Column::UpdatedAt,
        ])
        .to_owned()
}

#[async_trait::async_trait]
pub trait ServerSettingsRepository: Send + Sync {
    async fn get_all(&self) -> anyhow::Result<Vec<ServerSetting>>;
    async fn upsert(&self, input: UpsertServerSettingInput) -> anyhow::Result<ServerSetting>;
}

#[derive(Clone)]
pub struct ServerSettingsRepositoryImpl(DatabaseConnection);

impl ServerSettingsRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self(db)
    }
}

fn into_domain(model: server_settings::Model) -> ServerSetting {
    ServerSetting {
        id: model.id,
        setting_key: model.setting_key,
        value: model.value,
        description: model.description,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

#[async_trait::async_trait]
impl ServerSettingsRepository for ServerSettingsRepositoryImpl {
    async fn get_all(&self) -> anyhow::Result<Vec<ServerSetting>> {
        let settings = server_settings::Entity::find()
            .order_by_asc(server_settings::Column::SettingKey)
            .all(&self.0)
            .await?
            .into_iter()
            .map(into_domain)
            .collect::<Vec<_>>();

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
        let now = crate::db_time::now();

        let should_encrypt = input.setting_key == KEY_AI_OPENAI_API_KEY;
        let value = if should_encrypt {
            symmetric::encrypt(&input.value)
        } else {
            input.value.clone()
        };

        server_settings::Entity::insert(server_settings::ActiveModel {
            id: Set(id),
            setting_key: Set(input.setting_key.clone()),
            value: Set(value),
            description: Set(input.description),
            created_at: Set(now),
            updated_at: Set(now),
        })
        .on_conflict(upsert_on_conflict())
        .exec(&self.0)
        .await?;

        let setting = server_settings::Entity::find()
            .filter(server_settings::Column::SettingKey.eq(input.setting_key))
            .one(&self.0)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Server setting disappeared after upsert"))
            .map(into_domain)?;

        if setting.setting_key == KEY_AI_OPENAI_API_KEY && !setting.value.is_empty() {
            return Ok(ServerSetting {
                value: "***".to_string(),
                ..setting
            });
        }

        Ok(setting)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{DbBackend, EntityTrait, QueryTrait};

    #[test]
    fn upsert_uses_backend_specific_conflict_syntax() {
        let statement = server_settings::Entity::insert(server_settings::ActiveModel {
            id: Set(Uuid::nil()),
            setting_key: Set("key".to_string()),
            value: Set("value".to_string()),
            description: Set(None),
            created_at: Set(chrono::NaiveDateTime::default()),
            updated_at: Set(chrono::NaiveDateTime::default()),
        })
        .on_conflict(upsert_on_conflict());

        assert_eq!(
            statement.build(DbBackend::MySql).to_string(),
            "INSERT INTO `server_settings` (`id`, `setting_key`, `value`, `description`, `created_at`, `updated_at`) VALUES ('00000000-0000-0000-0000-000000000000', 'key', 'value', NULL, '1970-01-01 00:00:00.000000', '1970-01-01 00:00:00.000000') ON DUPLICATE KEY UPDATE `value` = VALUES(`value`), `description` = VALUES(`description`), `updated_at` = VALUES(`updated_at`)"
        );
        assert_eq!(
            statement.build(DbBackend::Postgres).to_string(),
            "INSERT INTO \"server_settings\" (\"id\", \"setting_key\", \"value\", \"description\", \"created_at\", \"updated_at\") VALUES ('00000000-0000-0000-0000-000000000000', 'key', 'value', NULL, '1970-01-01 00:00:00.000000', '1970-01-01 00:00:00.000000') ON CONFLICT (\"setting_key\") DO UPDATE SET \"value\" = \"excluded\".\"value\", \"description\" = \"excluded\".\"description\", \"updated_at\" = \"excluded\".\"updated_at\""
        );
    }
}
