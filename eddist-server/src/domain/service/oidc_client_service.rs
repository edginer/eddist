use std::collections::HashMap;

use base64::Engine;
use chacha20poly1305::{KeyInit, aead::Aead};
use openidconnect::{ClientId, ClientSecret, IssuerUrl, core::CoreProviderMetadata};

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
    let cipher = chacha20poly1305::ChaCha20Poly1305::new(
        md5::digest::generic_array::GenericArray::from_slice(&key),
    );

    let plaintext = if let Some(b64) = b64_secret.strip_prefix("v1:") {
        let data = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .unwrap();
        let (nonce_bytes, ciphertext) = data.split_at(12);
        cipher
            .decrypt(
                chacha20poly1305::Nonce::from_slice(nonce_bytes),
                chacha20poly1305::aead::Payload {
                    msg: ciphertext,
                    aad: b"",
                },
            )
            .unwrap()
    } else {
        // Legacy: zero nonce, ciphertext only
        let data = base64::engine::general_purpose::STANDARD
            .decode(b64_secret)
            .unwrap();
        cipher
            .decrypt(
                chacha20poly1305::Nonce::from_slice(&[0; 12]),
                chacha20poly1305::aead::Payload {
                    msg: &data,
                    aad: b"",
                },
            )
            .unwrap()
    };

    std::str::from_utf8(&plaintext).unwrap().to_string()
}
#[cfg(test)]
mod tests {
    use base64::Engine;

    use super::*;

    const TEST_KEY: &str = "a_very_secret_key_that_is_not_32_bytes!";

    #[test]
    fn test_decrypt_client_secret_v1() {
        unsafe { std::env::set_var("TINKER_SECRET", TEST_KEY) };

        let key = TEST_KEY.as_bytes().iter().take(32).copied().collect::<Vec<u8>>();
        let secret = "my_secret_client_secret";
        let nonce_bytes: [u8; 12] = rand::random();
        let cipher = chacha20poly1305::ChaCha20Poly1305::new(
            md5::digest::generic_array::GenericArray::from_slice(&key),
        );
        let ciphertext = cipher
            .encrypt(
                chacha20poly1305::Nonce::from_slice(&nonce_bytes),
                chacha20poly1305::aead::Payload {
                    msg: secret.as_bytes(),
                    aad: b"",
                },
            )
            .unwrap();
        let mut payload = nonce_bytes.to_vec();
        payload.extend_from_slice(&ciphertext);
        let b64 = format!(
            "v1:{}",
            base64::engine::general_purpose::STANDARD.encode(&payload)
        );

        assert_eq!(decrypt_client_secret(&b64), secret);
    }

    #[test]
    fn test_decrypt_client_secret_legacy() {
        unsafe { std::env::set_var("TINKER_SECRET", TEST_KEY) };

        let key = TEST_KEY.as_bytes().iter().take(32).copied().collect::<Vec<u8>>();
        let secret = "my_secret_client_secret";
        let cipher = chacha20poly1305::ChaCha20Poly1305::new(
            md5::digest::generic_array::GenericArray::from_slice(&key),
        );
        let ciphertext = cipher
            .encrypt(
                chacha20poly1305::Nonce::from_slice(&[0; 12]),
                chacha20poly1305::aead::Payload {
                    msg: secret.as_bytes(),
                    aad: b"",
                },
            )
            .unwrap();
        let b64 = base64::engine::general_purpose::STANDARD.encode(&ciphertext);

        assert_eq!(decrypt_client_secret(&b64), secret);
    }
}
