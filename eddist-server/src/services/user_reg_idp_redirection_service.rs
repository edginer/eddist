use std::sync::Arc;

use redis::{AsyncCommands, aio::ConnectionManager};

use crate::{
    config::AppConfig,
    domain::{service::oidc_client_service::OidcClientService, user::user_reg_state::UserRegState},
    repositories::idp_repository::IdpRepository,
};
use eddist_core::redis_keys::{user_reg_oauth2_authreq_key, user_reg_oauth2_state_key};

use super::AppService;

#[derive(Clone)]
pub struct UserRegIdpRedirectionService<I: IdpRepository> {
    idp_repo: I,
    redis_conn: ConnectionManager,
    config: Arc<AppConfig>,
}

impl<I: IdpRepository + Clone> UserRegIdpRedirectionService<I> {
    pub fn new(idp_repo: I, redis_conn: ConnectionManager, config: Arc<AppConfig>) -> Self {
        Self {
            idp_repo,
            redis_conn,
            config,
        }
    }
}

#[async_trait::async_trait]
impl<I: IdpRepository + Clone>
    AppService<
        UserRegIdpRedirectionServiceInput,
        UserRegIdpRedirectionServiceOutput,
        UserRegIdpRedirectionError,
    > for UserRegIdpRedirectionService<I>
{
    async fn execute(
        &self,
        input: UserRegIdpRedirectionServiceInput,
    ) -> Result<UserRegIdpRedirectionServiceOutput, UserRegIdpRedirectionError> {
        let mut redis_conn = self.redis_conn.clone();

        let idp_clients_svc = OidcClientService::new(self.idp_repo.clone(), self.config.clone());
        let idp_clients = idp_clients_svc.get_idp_clients().await?;

        let (_, idp_client) = idp_clients
            .get(&input.idp_name)
            .ok_or_else(|| anyhow::anyhow!("idp client not found: {}", input.idp_name))?;

        let Some(user_reg_state) = redis_conn
            .get_del::<_, Option<String>>(user_reg_oauth2_state_key(&input.user_reg_state_id))
            .await
            .map_err(anyhow::Error::from)?
        else {
            return Err(UserRegIdpRedirectionError::StateNotFound);
        };

        let user_reg_state =
            serde_json::from_str::<UserRegState>(&user_reg_state).map_err(anyhow::Error::from)?;

        let (authz_url, nonce, code_verifier) =
            idp_client.create_authz_request(input.user_reg_state_id.clone());

        let user_reg_state = UserRegState {
            idp_name: Some(input.idp_name),
            nonce: Some(nonce.secret().to_string()),
            code_verifier: Some(code_verifier.secret().to_string()),
            ..user_reg_state
        };

        redis_conn
            .set_ex::<_, _, ()>(
                user_reg_oauth2_authreq_key(&input.user_reg_state_id),
                serde_json::to_string(&user_reg_state).map_err(anyhow::Error::from)?,
                60 * 15,
            )
            .await
            .map_err(anyhow::Error::from)?;

        Ok(UserRegIdpRedirectionServiceOutput {
            authz_url: authz_url.to_string(),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UserRegIdpRedirectionError {
    #[error("user registration state not found")]
    StateNotFound,
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
