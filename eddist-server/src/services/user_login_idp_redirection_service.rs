use redis::{aio::ConnectionManager, AsyncCommands};
use uuid::Uuid;

use crate::{
    domain::{
        service::oidc_client_service::OidcClientService, user::user_login_state::UserLoginState,
    },
    repositories::idp_repository::IdpRepository,
    utils::redis::user_login_oauth2_authreq_key,
};

use super::AppService;

#[derive(Clone)]
pub struct UserLoginIdpRedirectionService<I: IdpRepository>(I, ConnectionManager);

impl<I: IdpRepository> UserLoginIdpRedirectionService<I> {
    pub fn new(idp_repo: I, redis_conn: ConnectionManager) -> Self {
        Self(idp_repo, redis_conn)
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
        let mut redis_conn = self.1.clone();

        let idp_clients_svc = OidcClientService::new(self.0.clone(), self.1.clone());
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

        redis_conn
            .set_ex::<_, _, String>(
                user_login_oauth2_authreq_key(&user_login_state.user_login_state_id),
                serde_json::to_string(&user_login_state)?,
                60 * 15,
            )
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
