use std::env;

use openidconnect::{
    core::{
        CoreClient, CoreGenderClaim, CoreJwsSigningAlgorithm, CoreProviderMetadata,
        CoreResponseType,
    },
    AuthenticationFlow, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    EmptyAdditionalClaims, IdTokenClaims, Nonce, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl,
    TokenResponse,
};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcClient {
    client_id: ClientId,
    client_secret: ClientSecret,
    metadata: CoreProviderMetadata,
    redirect_url: RedirectUrl,
}

impl OidcClient {
    pub async fn new(
        client_id: ClientId,
        client_secret: openidconnect::ClientSecret,
        oidc_config: CoreProviderMetadata,
    ) -> Self {
        let mut id_token_signing_alg_values_supported =
            oidc_config.id_token_signing_alg_values_supported().clone();
        id_token_signing_alg_values_supported.push(CoreJwsSigningAlgorithm::HmacSha256); // for some IdP that doesn't show correct alg in metadata

        let metadata = oidc_config
            .set_id_token_signing_alg_values_supported(id_token_signing_alg_values_supported);
        Self {
            client_id,
            client_secret,
            metadata,
            redirect_url: RedirectUrl::new(format!(
                "{}/user/auth/callback",
                env::var("BASE_URL").unwrap()
            ))
            .unwrap(),
        }
    }

    pub fn create_authz_request(&self, state: String) -> (Url, Nonce, PkceCodeVerifier) {
        let client = CoreClient::from_provider_metadata(
            self.metadata.clone(),
            self.client_id.clone(),
            None, // This process doesn't require a client secret
        );
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let binding = client.set_redirect_uri(self.redirect_url.clone());
        let authz_req = binding
            .authorize_url(
                AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
                || CsrfToken::new(state),
                || Nonce::new_random(),
            )
            .set_pkce_challenge(pkce_challenge);

        let (auth_url, _, nonce) = authz_req.url();

        (auth_url, nonce, pkce_verifier)
    }

    pub async fn exchange_code(
        &self,
        authz_code: AuthorizationCode,
        pkce_verifier: PkceCodeVerifier,
        nonce: Nonce,
    ) -> IdTokenClaims<EmptyAdditionalClaims, CoreGenderClaim> {
        let client = CoreClient::from_provider_metadata(
            self.metadata.clone(),
            self.client_id.clone(),
            Some(self.client_secret.clone()),
        );

        let res = client
            .exchange_code(authz_code)
            .unwrap()
            .set_pkce_verifier(pkce_verifier)
            .set_redirect_uri(std::borrow::Cow::Borrowed(&self.redirect_url))
            .request_async(&reqwest::Client::new())
            .await
            .unwrap();

        let id_token = res.id_token().unwrap();

        let claims = id_token
            .claims(&client.id_token_verifier(), &nonce)
            .unwrap();

        claims.clone()
    }
}
