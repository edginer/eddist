use redis::{aio::ConnectionManager, AsyncCommands};

use crate::{
    domain::{
        service::oidc_client_service::OidcClientService, user::user_link_state::UserLinkState,
    },
    repositories::idp_repository::IdpRepository,
    utils::redis::{user_link_oauth2_authreq_key, user_link_oauth2_state_key},
};

use super::AppService;

#[derive(Clone)]
pub struct UserLinkIdpRedirectionService<I: IdpRepository> {
    idp_repo: I,
    redis_conn: ConnectionManager,
}

impl<I: IdpRepository + Clone> UserLinkIdpRedirectionService<I> {
    pub fn new(idp_repo: I, redis_conn: ConnectionManager) -> Self {
        Self {
            idp_repo,
            redis_conn,
        }
    }
}

#[async_trait::async_trait]
impl<I: IdpRepository + Clone>
    AppService<UserLinkIdpRedirectionServiceInput, UserLinkIdpRedirectionServiceOutput>
    for UserLinkIdpRedirectionService<I>
{
    async fn execute(
        &self,
        input: UserLinkIdpRedirectionServiceInput,
    ) -> anyhow::Result<UserLinkIdpRedirectionServiceOutput> {
        let mut redis_conn = self.redis_conn.clone();

        let idp_clients_svc =
            OidcClientService::new(self.idp_repo.clone(), self.redis_conn.clone());
        let idp_clients = idp_clients_svc.get_idp_clients().await?;

        let (_, idp_client) = idp_clients
            .get(&input.idp_name)
            .ok_or_else(|| anyhow::anyhow!("idp client not found: {}", input.idp_name))?;

        let Ok(user_link_state) = redis_conn
            .get_del::<_, String>(user_link_oauth2_state_key(&input.user_link_state_id))
            .await
        else {
            return Err(anyhow::anyhow!("user_link_state_id not found"));
        };

        let user_link_state = serde_json::from_str::<UserLinkState>(&user_link_state)?;

        let (authz_url, nonce, code_verifier) =
            idp_client.create_authz_request(input.user_link_state_id.clone());

        let user_link_state = UserLinkState {
            idp_name: Some(input.idp_name),
            nonce: Some(nonce.secret().to_string()),
            code_verifier: Some(code_verifier.secret().to_string()),
            ..user_link_state
        };

        redis_conn
            .set_ex::<_, _, ()>(
                user_link_oauth2_authreq_key(&input.user_link_state_id),
                serde_json::to_string(&user_link_state)?,
                60 * 15, // 15 minutes
            )
            .await?;

        Ok(UserLinkIdpRedirectionServiceOutput {
            authz_url: authz_url.to_string(),
        })
    }
}

pub struct UserLinkIdpRedirectionServiceInput {
    pub idp_name: String,
    pub user_link_state_id: String,
}

pub struct UserLinkIdpRedirectionServiceOutput {
    pub authz_url: String,
}
