#[cfg(not(feature = "backend-postgres"))]
use sqlx::MySqlPool;
#[cfg(feature = "backend-postgres")]
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::user::idp::Idp;

#[async_trait::async_trait]
pub trait IdpRepository: Send + Sync + 'static {
    async fn get_idps(&self) -> anyhow::Result<Vec<Idp>>;
}

#[cfg(not(feature = "backend-postgres"))]
#[derive(Debug, Clone)]
pub struct IdpRepositoryImpl {
    pool: MySqlPool,
}

#[cfg(not(feature = "backend-postgres"))]
impl IdpRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[cfg(not(feature = "backend-postgres"))]
#[async_trait::async_trait]
impl IdpRepository for IdpRepositoryImpl {
    async fn get_idps(&self) -> anyhow::Result<Vec<Idp>> {
        let idps = sqlx::query_as!(
            Idp,
            r#"
            SELECT 
                id as "id: Uuid",
                idp_name,
                oidc_config_url,
                idp_display_name,
                idp_logo_svg,
                client_id,
                client_secret,
                enabled as "enabled: bool"
            FROM idps
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(idps)
    }
}

#[cfg(feature = "backend-postgres")]
#[derive(Debug, Clone)]
pub struct IdpRepositoryPgImpl {
    pool: PgPool,
}

#[cfg(feature = "backend-postgres")]
impl IdpRepositoryPgImpl {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[cfg(feature = "backend-postgres")]
#[async_trait::async_trait]
impl IdpRepository for IdpRepositoryPgImpl {
    async fn get_idps(&self) -> anyhow::Result<Vec<Idp>> {
        let idps = sqlx::query_as::<_, Idp>(
            r#"
            SELECT id, idp_name, oidc_config_url, idp_display_name, idp_logo_svg,
                   client_id, client_secret, enabled
            FROM idps
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(idps)
    }
}
