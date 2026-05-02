use chrono::Utc;
#[cfg(not(feature = "backend-postgres"))]
use sqlx::{MySqlPool, query, query_as};
use uuid::Uuid;

use crate::models::server_settings::{ServerSetting, UpsertServerSettingInput};

#[cfg(feature = "backend-postgres")]
#[derive(Debug, sqlx::FromRow)]
struct ServerSettingPg {
    pub id: Uuid,
    pub setting_key: String,
    pub value: String,
    pub description: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
}

#[cfg(feature = "backend-postgres")]
impl From<ServerSettingPg> for ServerSetting {
    fn from(r: ServerSettingPg) -> Self {
        Self {
            id: r.id,
            setting_key: r.setting_key,
            value: r.value,
            description: r.description,
            created_at: r.created_at.naive_utc(),
            updated_at: r.updated_at.naive_utc(),
        }
    }
}

#[async_trait::async_trait]
pub trait ServerSettingsRepository: Send + Sync {
    async fn get_all(&self) -> anyhow::Result<Vec<ServerSetting>>;
    async fn upsert(&self, input: UpsertServerSettingInput) -> anyhow::Result<ServerSetting>;
}

#[cfg(not(feature = "backend-postgres"))]
#[derive(Clone)]
pub struct ServerSettingsRepositoryImpl(MySqlPool);

#[cfg(not(feature = "backend-postgres"))]
impl ServerSettingsRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self(pool)
    }
}

#[cfg(not(feature = "backend-postgres"))]
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

        Ok(settings)
    }

    async fn upsert(&self, input: UpsertServerSettingInput) -> anyhow::Result<ServerSetting> {
        let id = Uuid::now_v7();
        let now = Utc::now().naive_utc();

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
            input.value,
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

        Ok(setting)
    }
}

#[cfg(feature = "backend-postgres")]
#[derive(Clone)]
pub struct ServerSettingsRepositoryPgImpl(sqlx::PgPool);

#[cfg(feature = "backend-postgres")]
impl ServerSettingsRepositoryPgImpl {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self(pool)
    }
}

#[cfg(feature = "backend-postgres")]
#[async_trait::async_trait]
impl ServerSettingsRepository for ServerSettingsRepositoryPgImpl {
    async fn get_all(&self) -> anyhow::Result<Vec<ServerSetting>> {
        let rows = sqlx::query_as::<_, ServerSettingPg>(
            r#"
            SELECT id, setting_key, value, description, created_at, updated_at
            FROM server_settings
            ORDER BY setting_key ASC
            "#,
        )
        .fetch_all(&self.0)
        .await?;

        Ok(rows.into_iter().map(ServerSetting::from).collect())
    }

    async fn upsert(&self, input: UpsertServerSettingInput) -> anyhow::Result<ServerSetting> {
        let id = Uuid::now_v7();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO server_settings (id, setting_key, value, description, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (setting_key) DO UPDATE SET
                value = EXCLUDED.value,
                description = EXCLUDED.description,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(id)
        .bind(&input.setting_key)
        .bind(&input.value)
        .bind(&input.description)
        .bind(now)
        .bind(now)
        .execute(&self.0)
        .await?;

        let row = sqlx::query_as::<_, ServerSettingPg>(
            r#"
            SELECT id, setting_key, value, description, created_at, updated_at
            FROM server_settings
            WHERE setting_key = $1
            "#,
        )
        .bind(&input.setting_key)
        .fetch_one(&self.0)
        .await?;

        Ok(ServerSetting::from(row))
    }
}
