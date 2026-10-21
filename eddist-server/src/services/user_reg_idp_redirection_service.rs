use std::sync::Arc;

use crate::{
    domain::{
        service::oidc_client_service::{MetadataFreshness, OidcClientRegistry},
        user::user_reg_state::UserRegState,
    },
    stores::oidc_state_store::OidcStateStore,
};

use super::AppService;

#[derive(Clone)]
pub struct UserRegIdpRedirectionService {
    oidc_clients: Arc<OidcClientRegistry>,
    oidc_state: Arc<dyn OidcStateStore>,
}

impl UserRegIdpRedirectionService {
    pub fn new(oidc_clients: Arc<OidcClientRegistry>, oidc_state: Arc<dyn OidcStateStore>) -> Self {
        Self {
            oidc_clients,
            oidc_state,
        }
    }
}

#[async_trait::async_trait]
impl
    AppService<
        UserRegIdpRedirectionServiceInput,
        UserRegIdpRedirectionServiceOutput,
        UserRegIdpRedirectionError,
    > for UserRegIdpRedirectionService
{
    async fn execute(
        &self,
        input: UserRegIdpRedirectionServiceInput,
    ) -> Result<UserRegIdpRedirectionServiceOutput, UserRegIdpRedirectionError> {
        let Some((_, idp_client)) = self
            .oidc_clients
            .get(&input.idp_name, MetadataFreshness::Cached)
            .await?
        else {
            return Err(UserRegIdpRedirectionError::UnknownIdp);
        };

        let Some(user_reg_state) = self
            .oidc_state
            .take_user_reg_state(&input.user_reg_state_id)
            .await?
        else {
            return Err(UserRegIdpRedirectionError::StateNotFound);
        };

        let (authz_url, nonce, code_verifier) =
            idp_client.create_authz_request(input.user_reg_state_id.clone());

        let user_reg_state = UserRegState {
            idp_name: Some(input.idp_name),
            nonce: Some(nonce.secret().to_string()),
            code_verifier: Some(code_verifier.secret().to_string()),
            ..user_reg_state
        };

        self.oidc_state
            .save_user_reg_authreq(&input.user_reg_state_id, &user_reg_state)
            .await?;

        Ok(UserRegIdpRedirectionServiceOutput {
            authz_url: authz_url.to_string(),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UserRegIdpRedirectionError {
    #[error("user registration state not found")]
    StateNotFound,
    #[error("unknown idp")]
    UnknownIdp,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub struct UserRegIdpRedirectionServiceInput {
    pub idp_name: String,
    pub user_reg_state_id: String,
}

pub struct UserRegIdpRedirectionServiceOutput {
    pub authz_url: String,
}
