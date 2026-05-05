use std::collections::HashMap;

use eddist_core::symmetric;
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
    symmetric::decrypt(b64_secret).expect("failed to decrypt client_secret")
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decrypt_client_secret_round_trip() {
        unsafe {
            std::env::set_var("TINKER_SECRET", "a_very_secret_key_that_is_not_32_bytes!")
        };
        let secret = "my_secret_client_secret";
        let encrypted = symmetric::encrypt(secret);
        assert_eq!(decrypt_client_secret(&encrypted), secret);
    }
}
