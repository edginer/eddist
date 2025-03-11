use redis::{aio::ConnectionManager, AsyncCommands};

use crate::{
    domain::{service::oidc_client_service::OidcClientService, user::user_reg_state::UserRegState},
    repositories::idp_repository::IdpRepository,
};

use super::AppService;

#[derive(Clone)]
pub struct UserRegIdpRedirectionService<I: IdpRepository> {
    idp_repo: I,
    redis_conn: ConnectionManager,
}

impl<I: IdpRepository + Clone> UserRegIdpRedirectionService<I> {
    pub fn new(idp_repo: I, redis_conn: ConnectionManager) -> Self {
        Self {
            idp_repo,
            redis_conn,
        }
    }
}

#[async_trait::async_trait]
impl<I: IdpRepository + Clone>
    AppService<UserRegIdpRedirectionServiceInput, UserRegIdpRedirectionServiceOutput>
    for UserRegIdpRedirectionService<I>
{
    async fn execute(
        &self,
        input: UserRegIdpRedirectionServiceInput,
    ) -> anyhow::Result<UserRegIdpRedirectionServiceOutput> {
        let mut redis_conn = self.redis_conn.clone();

        let idp_clients_svc =
            OidcClientService::new(self.idp_repo.clone(), self.redis_conn.clone());
        let idp_clients = idp_clients_svc.get_idp_clients().await?;

        let (_, idp_client) = idp_clients
            .get(&input.idp_name)
            .ok_or_else(|| anyhow::anyhow!("idp client not found: {}", input.idp_name))?;

        let Ok(user_reg_state) = redis_conn
            .get_del::<_, String>(format!("userreg:oauth2:state:{}", input.user_reg_state_id))
            .await
        else {
            return Err(anyhow::anyhow!("user_reg_state_id not found"));
        };

        let user_reg_state = serde_json::from_str::<UserRegState>(&user_reg_state)?;

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
                format!("userreg:oauth2:authreq:{}", input.user_reg_state_id),
                serde_json::to_string(&user_reg_state)?,
                60 * 15,
            )
            .await?;

        Ok(UserRegIdpRedirectionServiceOutput {
            authz_url: authz_url.to_string(),
        })
    }
}

pub struct UserRegIdpRedirectionServiceInput {
    pub idp_name: String,
    pub user_reg_state_id: String,
}

pub struct UserRegIdpRedirectionServiceOutput {
    pub authz_url: String,
}
