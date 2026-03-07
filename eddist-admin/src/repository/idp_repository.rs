use base64::Engine;
use chacha20poly1305::{aead::Aead, KeyInit};
use sqlx::{query, query_as, MySqlPool};
use uuid::Uuid;

use crate::models::idp::{CreateIdpInput, Idp, UpdateIdpInput};

#[async_trait::async_trait]
pub trait IdpAdminRepository: Send + Sync {
    async fn get_all(&self) -> anyhow::Result<Vec<Idp>>;
    async fn get_by_id(&self, id: Uuid) -> anyhow::Result<Option<Idp>>;
    async fn create(&self, input: CreateIdpInput) -> anyhow::Result<Idp>;
    async fn update(&self, id: Uuid, input: UpdateIdpInput) -> anyhow::Result<Idp>;
    async fn delete(&self, id: Uuid) -> anyhow::Result<()>;
}

#[derive(Clone)]
pub struct IdpAdminRepositoryImpl(MySqlPool);

impl IdpAdminRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self(pool)
    }
}

fn encrypt_client_secret(plain_secret: &str) -> String {
    let key = std::env::var("TINKER_SECRET").unwrap();
    let key = key.as_bytes().iter().take(32).copied().collect::<Vec<u8>>();

    let encrypted = chacha20poly1305::ChaCha20Poly1305::new(
        md5::digest::generic_array::GenericArray::from_slice(&key),
    )
    .encrypt(
        chacha20poly1305::Nonce::from_slice(&[0; 12]),
        chacha20poly1305::aead::Payload {
            msg: plain_secret.as_bytes(),
            aad: b"",
        },
    )
    .unwrap();

    base64::engine::general_purpose::STANDARD.encode(&encrypted)
}

#[async_trait::async_trait]
impl IdpAdminRepository for IdpAdminRepositoryImpl {
    async fn get_all(&self) -> anyhow::Result<Vec<Idp>> {
        let idps = query_as!(
            Idp,
            r#"
            SELECT
                id AS "id: Uuid",
                idp_name,
                idp_display_name,
                idp_logo_svg,
                oidc_config_url,
                client_id,
                enabled AS "enabled: bool"
            FROM idps
            ORDER BY idp_name
            "#
        )
        .fetch_all(&self.0)
        .await?;

        Ok(idps)
    }

    async fn get_by_id(&self, id: Uuid) -> anyhow::Result<Option<Idp>> {
        let idp = query_as!(
            Idp,
            r#"
            SELECT
                id AS "id: Uuid",
                idp_name,
                idp_display_name,
                idp_logo_svg,
                oidc_config_url,
                client_id,
                enabled AS "enabled: bool"
            FROM idps
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(&self.0)
        .await?;

        Ok(idp)
    }

    async fn create(&self, input: CreateIdpInput) -> anyhow::Result<Idp> {
        let id = Uuid::now_v7();
        let encrypted_secret = encrypt_client_secret(&input.client_secret);

        query!(
            r#"
            INSERT INTO idps (id, idp_name, idp_display_name, idp_logo_svg, oidc_config_url, client_id, client_secret, enabled)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            id,
            input.idp_name,
            input.idp_display_name,
            input.idp_logo_svg,
            input.oidc_config_url,
            input.client_id,
            encrypted_secret,
            input.enabled,
        )
        .execute(&self.0)
        .await?;

        Ok(Idp {
            id,
            idp_name: input.idp_name,
            idp_display_name: input.idp_display_name,
            idp_logo_svg: input.idp_logo_svg,
            oidc_config_url: input.oidc_config_url,
            client_id: input.client_id,
            enabled: input.enabled,
        })
    }

    async fn update(&self, id: Uuid, input: UpdateIdpInput) -> anyhow::Result<Idp> {
        let current = self
            .get_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("IdP not found"))?;

        let idp_display_name = input.idp_display_name.unwrap_or(current.idp_display_name);
        let idp_logo_svg = if input.idp_logo_svg.is_some() {
            input.idp_logo_svg
        } else {
            current.idp_logo_svg
        };
        let oidc_config_url = input.oidc_config_url.unwrap_or(current.oidc_config_url);
        let client_id = input.client_id.unwrap_or(current.client_id);
        let enabled = input.enabled.unwrap_or(current.enabled);

        // Only re-encrypt if a new client_secret is provided
        if let Some(ref new_secret) = input.client_secret {
            let encrypted_secret = encrypt_client_secret(new_secret);
            query!(
                r#"
                UPDATE idps
                SET idp_display_name = ?, idp_logo_svg = ?, oidc_config_url = ?, client_id = ?, client_secret = ?, enabled = ?
                WHERE id = ?
                "#,
                idp_display_name,
                idp_logo_svg,
                oidc_config_url,
                client_id,
                encrypted_secret,
                enabled,
                id,
            )
            .execute(&self.0)
            .await?;
        } else {
            query!(
                r#"
                UPDATE idps
                SET idp_display_name = ?, idp_logo_svg = ?, oidc_config_url = ?, client_id = ?, enabled = ?
                WHERE id = ?
                "#,
                idp_display_name,
                idp_logo_svg,
                oidc_config_url,
                client_id,
                enabled,
                id,
            )
            .execute(&self.0)
            .await?;
        }

        Ok(Idp {
            id,
            idp_name: current.idp_name,
            idp_display_name,
            idp_logo_svg,
            oidc_config_url,
            client_id,
            enabled,
        })
    }

    async fn delete(&self, id: Uuid) -> anyhow::Result<()> {
        query!(
            r#"
            DELETE FROM idps
            WHERE id = ?
            "#,
            id
        )
        .execute(&self.0)
        .await?;

        Ok(())
    }
}
