use std::sync::Arc;

use uuid::Uuid;

use crate::{
    config::AppConfig,
    domain::{
        service::oidc_client_service::OidcClientService, user::user_login_state::UserLoginState,
    },
    repositories::idp_repository::IdpRepository,
    stores::oidc_state_store::OidcStateStore,
};

use super::AppService;

#[derive(Clone)]
pub struct UserLoginIdpRedirectionService<I: IdpRepository>(
    I,
    Arc<dyn OidcStateStore>,
    Arc<AppConfig>,
);

impl<I: IdpRepository> UserLoginIdpRedirectionService<I> {
    pub fn new(idp_repo: I, oidc_state: Arc<dyn OidcStateStore>, config: Arc<AppConfig>) -> Self {
        Self(idp_repo, oidc_state, config)
    }
}

#[async_trait::async_trait]
impl<I: IdpRepository + Clone>
    AppService<UserLoginIdpRedirectionServiceInput, UserLoginIdpRedirectionServiceOutput>
    for UserLoginIdpRedirectionService<I>
{
    async fn execute(
        &self,
        input: UserLoginIdpRedirectionServiceInput,
    ) -> anyhow::Result<UserLoginIdpRedirectionServiceOutput> {
        let idp_clients_svc = OidcClientService::new(self.0.clone(), self.2.clone());
        let idp_clients = idp_clients_svc.get_idp_clients().await?;

        let (_, idp_client) = idp_clients
            .get(&input.idp_name)
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

        self.1.save_user_login_authreq(&user_login_state).await?;

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
