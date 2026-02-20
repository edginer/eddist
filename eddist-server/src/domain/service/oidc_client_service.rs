use std::collections::HashMap;

use base64::Engine;
use chacha20poly1305::{aead::Aead, KeyInit};
use openidconnect::{core::CoreProviderMetadata, ClientId, ClientSecret, IssuerUrl};

use crate::{
    domain::user::idp::Idp, external::oidc_client::OidcClient,
    repositories::idp_repository::IdpRepository,
};

#[derive(Clone)]
pub struct OidcClientService<T: IdpRepository> {
    idp_repo: T,
}

impl<T: IdpRepository + Clone> OidcClientService<T> {
    pub fn new(idp_repo: T) -> Self {
        Self { idp_repo }
    }

    pub async fn get_idp_clients(&self) -> anyhow::Result<HashMap<String, (Idp, OidcClient)>> {
        let mut idps = HashMap::new();

        let http_client = reqwest::Client::new();
        for idp in self.idp_repo.get_idps().await? {
            let issuer_url = IssuerUrl::new(
                idp.oidc_config_url
                    .trim_end_matches("/.well-known/openid-configuration")
                    .to_string(),
            )
            .unwrap();
            let metadata = CoreProviderMetadata::discover_async(issuer_url, &http_client)
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
