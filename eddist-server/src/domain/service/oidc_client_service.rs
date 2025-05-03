use std::collections::HashMap;

use base64::Engine;
use chacha20poly1305::{KeyInit, aead::Aead};
use eddist_core::cache_aside::{self, AsCache, ToCache};
use openidconnect::{ClientId, ClientSecret, core::CoreProviderMetadata};
use redis::aio::ConnectionManager;
use serde::{Deserialize, Serialize};

use crate::{
    domain::user::idp::Idp, external::oidc_client::OidcClient,
    repositories::idp_repository::IdpRepository,
};

#[derive(Clone)]
pub struct OidcClientService<T: IdpRepository> {
    idp_repo: T,
    redis_conn: ConnectionManager,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IdPCache {
    idps: HashMap<String, (Idp, OidcClient)>,
    expired_at: u64,
}

impl AsCache<HashMap<String, (Idp, OidcClient)>> for IdPCache {
    fn expired_at(&self) -> u64 {
        self.expired_at
    }

    fn get(self) -> HashMap<String, (Idp, OidcClient)> {
        self.idps
    }
}

impl ToCache<HashMap<String, (Idp, OidcClient)>, IdPCache> for HashMap<String, (Idp, OidcClient)> {
    fn into_cache(self, expired_at: u64) -> IdPCache {
        IdPCache {
            idps: self,
            expired_at,
        }
    }
}

impl<T: IdpRepository + Clone> OidcClientService<T> {
    pub fn new(idp_repo: T, redis_conn: ConnectionManager) -> Self {
        Self {
            idp_repo,
            redis_conn,
        }
    }

    pub async fn get_idp_clients(&self) -> anyhow::Result<HashMap<String, (Idp, OidcClient)>> {
        let repo = self.idp_repo.clone();
        let expired_at = chrono::Utc::now().timestamp() as u64 + 300;

        let idp = cache_aside::cache_aside(
            "user",
            "idp_configs",
            Box::new(self.redis_conn.clone()),
            expired_at,
            || {
                Box::pin(async move {
                    let mut idps = HashMap::new();

                    for idp in repo.get_idps().await? {
                        let metadata = reqwest::get(&idp.oidc_config_url)
                            .await
                            .unwrap()
                            .json::<CoreProviderMetadata>()
                            .await
                            .unwrap();
                        let idp_name = idp.idp_name.clone();
                        let (client_id, client_secret) = (
                            ClientId::new(idp.client_id.clone()),
                            ClientSecret::new({
                                std::env::var("CLIENT_SECRET_SYMMETRIC_ENCRYPTION")
                                    .map(|b| {
                                        let b = b.parse::<bool>().unwrap();
                                        if b {
                                            decrypt_client_secret(&idp.client_secret)
                                        } else {
                                            idp.client_secret.clone()
                                        }
                                    })
                                    .unwrap_or(idp.client_secret.clone())
                            }),
                        );

                        let idp_value = (
                            idp,
                            OidcClient::new(client_id, client_secret, metadata).await,
                        );
                        idps.insert(idp_name, idp_value);
                    }

                    Ok(idps)
                })
            },
        )
        .await?;

        Ok(idp)
    }
}

fn decrypt_client_secret(b64_secret: &str) -> String {
    let key = std::env::var("TINKER_SECRET").unwrap();
    let key = key.as_bytes().iter().take(32).copied().collect::<Vec<u8>>();

    let secret = base64::engine::general_purpose::STANDARD
        .decode(b64_secret)
        .unwrap();

    let secret = chacha20poly1305::ChaCha20Poly1305::new(
        md5::digest::generic_array::GenericArray::from_slice(&key),
    )
    .decrypt(
        chacha20poly1305::Nonce::from_slice(&[0; 12]),
        chacha20poly1305::aead::Payload {
            msg: &secret,
            aad: b"",
        },
    )
    .unwrap();

    std::str::from_utf8(&secret).unwrap().to_string()
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decrypt_client_secret() {
        // Set up the environment variable for the key
        std::env::set_var("TINKER_SECRET", "a_very_secret_key_that_is_not_32_bytes!");

        // Encrypt a sample secret
        let key = std::env::var("TINKER_SECRET").unwrap();
        let key = key.as_bytes().iter().take(32).copied().collect::<Vec<u8>>();
        let secret = "my_secret_client_secret";
        let cipher = chacha20poly1305::ChaCha20Poly1305::new(
            md5::digest::generic_array::GenericArray::from_slice(&key),
        );
        let encrypted_secret = cipher
            .encrypt(
                chacha20poly1305::Nonce::from_slice(&[0; 12]),
                chacha20poly1305::aead::Payload {
                    msg: secret.as_bytes(),
                    aad: b"",
                },
            )
            .unwrap();
        let b64_encrypted_secret =
            base64::engine::general_purpose::STANDARD.encode(&encrypted_secret);

        // Decrypt the secret and verify it matches the original
        let decrypted_secret = decrypt_client_secret(&b64_encrypted_secret);
        assert_eq!(decrypted_secret, secret);
    }
}
