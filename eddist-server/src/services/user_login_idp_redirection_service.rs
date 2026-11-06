use std::sync::Arc;

use uuid::Uuid;

use crate::{
    domain::{
        service::oidc_client_service::{MetadataFreshness, OidcClientRegistry},
        user::user_login_state::UserLoginState,
    },
    stores::oidc_state_store::OidcStateStore,
};

use super::AppService;

#[derive(Clone)]
pub struct UserLoginIdpRedirectionService {
    oidc_clients: Arc<OidcClientRegistry>,
    oidc_state: Arc<dyn OidcStateStore>,
}

impl UserLoginIdpRedirectionService {
    pub fn new(oidc_clients: Arc<OidcClientRegistry>, oidc_state: Arc<dyn OidcStateStore>) -> Self {
        Self {
            oidc_clients,
            oidc_state,
        }
    }
}

#[async_trait::async_trait]
impl AppService<UserLoginIdpRedirectionServiceInput, UserLoginIdpRedirectionServiceOutput>
    for UserLoginIdpRedirectionService
{
    async fn execute(
        &self,
        input: UserLoginIdpRedirectionServiceInput,
    ) -> anyhow::Result<UserLoginIdpRedirectionServiceOutput> {
        let (_, idp_client) = self
            .oidc_clients
            .get(&input.idp_name, MetadataFreshness::Cached)
            .await?
            .ok_or_else(|| anyhow::anyhow!("idp client not found: {}", input.idp_name))?;

        let user_login_state_id = Uuid::now_v7();

        let (authz_url, nonce, code_verifier) =
            idp_client.create_authz_request(user_login_state_id.to_string());

        let user_login_state = UserLoginState {
            idp_name: input.idp_name,
            nonce: nonce.secret().to_string(),
            code_verifier: code_verifier.secret().to_string(),
            user_login_state_id: user_login_state_id.to_string(),
        };

        self.oidc_state
            .save_user_login_authreq(&user_login_state)
            .await?;

        Ok(UserLoginIdpRedirectionServiceOutput {
            authz_url: authz_url.to_string(),
            user_login_state_id: user_login_state.user_login_state_id,
        })
    }
}

pub struct UserLoginIdpRedirectionServiceInput {
    pub idp_name: String,
}

pub struct UserLoginIdpRedirectionServiceOutput {
    pub authz_url: String,
    pub user_login_state_id: String,
}
