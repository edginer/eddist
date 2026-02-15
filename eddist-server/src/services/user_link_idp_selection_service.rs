use redis::{aio::ConnectionManager, AsyncCommands};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    domain::user::user_link_state::UserLinkState,
    repositories::{bbs_repository::BbsRepository, idp_repository::IdpRepository},
    utils::redis::user_link_oauth2_state_key,
};

use super::AppService;

#[derive(Clone)]
pub struct UserLinkIdpSelectionService<I: IdpRepository, B: BbsRepository> {
    idp_repo: I,
    bbs_repo: B,
    redis_conn: ConnectionManager,
}

impl<I: IdpRepository + Clone, B: BbsRepository + Clone> UserLinkIdpSelectionService<I, B> {
    pub fn new(idp_repo: I, bbs_repo: B, redis_conn: ConnectionManager) -> Self {
        Self {
            idp_repo,
            bbs_repo,
            redis_conn,
        }
    }
}

#[async_trait::async_trait]
impl<I: IdpRepository + Clone, B: BbsRepository + Clone>
    AppService<UserLinkIdpSelectionServiceInput, UserLinkIdpSelectionServiceOutput>
    for UserLinkIdpSelectionService<I, B>
{
    async fn execute(
        &self,
        input: UserLinkIdpSelectionServiceInput,
    ) -> anyhow::Result<UserLinkIdpSelectionServiceOutput> {
        let mut redis_conn = self.redis_conn.clone();

        // Verify the token exists and is valid
        let authed_token = self
            .bbs_repo
            .get_authed_token(&input.token)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Token not found"))?;

        if !authed_token.validity {
            return Err(anyhow::anyhow!("Token is not valid"));
        }

        // Check if the token is already linked to a user
        if authed_token.registered_user_id.is_some() {
            return Ok(UserLinkIdpSelectionServiceOutput::AlreadyLinked);
        }

        // Get available IdPs
        let idps = self.idp_repo.get_idps().await?;
        let available_idps = idps
            .into_iter()
            .filter(|idp| idp.enabled)
            .map(|idp| AvailableIdp {
                idp_name: idp.idp_name,
                idp_display_name: idp.idp_display_name,
                idp_logo_svg: idp.idp_logo_svg,
            })
            .collect::<Vec<_>>();

        if available_idps.is_empty() {
            return Err(anyhow::anyhow!("No IdPs available"));
        }

        // Create state and store in Redis
        let state_cookie = Uuid::now_v7().to_string();
        let user_link_state = UserLinkState {
            authed_token: input.token,
            authed_token_id: authed_token.id.to_string(),
            ..UserLinkState::default()
        };

        redis_conn
            .set_ex::<_, _, ()>(
                user_link_oauth2_state_key(&state_cookie),
                serde_json::to_string(&user_link_state)?,
                60 * 3, // 3 minutes
            )
            .await?;

        Ok(UserLinkIdpSelectionServiceOutput::NotLinked {
            available_idps,
            state_cookie,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AvailableIdp {
    pub idp_name: String,
    pub idp_display_name: String,
    pub idp_logo_svg: Option<String>,
}

pub struct UserLinkIdpSelectionServiceInput {
    pub token: String,
}

pub enum UserLinkIdpSelectionServiceOutput {
    AlreadyLinked,
    NotLinked {
        available_idps: Vec<AvailableIdp>,
        state_cookie: String,
    },
}
