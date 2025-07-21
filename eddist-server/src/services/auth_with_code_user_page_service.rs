use chrono::Utc;
use metrics::counter;
use redis::{aio::ConnectionManager, AsyncCommands};
use sqlx::MySql;

use crate::{
    repositories::{bbs_repository::BbsRepository, user_repository::UserRepository},
    utils::{redis::user_session_key, TransactionRepository},
};

use super::AppService;

#[derive(Clone)]
pub struct AuthWithCodeUserPageService<U: UserRepository, B: BbsRepository>(
    U,
    B,
    ConnectionManager,
);

impl<U: UserRepository + Clone, B: BbsRepository + Clone> AuthWithCodeUserPageService<U, B> {
    pub fn new(user_repo: U, bbs_repo: B, redis_conn: ConnectionManager) -> Self {
        Self(user_repo, bbs_repo, redis_conn)
    }
}

#[async_trait::async_trait]
impl<U: UserRepository + TransactionRepository<MySql> + Clone, B: BbsRepository + Clone>
    AppService<AuthWithCodeUserPageServiceInput, AuthWithCodeUserPageServiceOutput>
    for AuthWithCodeUserPageService<U, B>
{
    async fn execute(
        &self,
        input: AuthWithCodeUserPageServiceInput,
    ) -> anyhow::Result<AuthWithCodeUserPageServiceOutput> {
        let mut redis_conn = self.2.clone();
        let user_id = redis_conn
            .get::<_, String>(user_session_key(&input.user_sid))
            .await?;

        let Some(user) = self.0.get_user_by_id(user_id.parse().unwrap()).await? else {
            return Err(anyhow::anyhow!("user not found"));
        };

        if !user.enabled {
            return Err(anyhow::anyhow!("user not enabled"));
        }

        let mut tokens = self
            .1
            .get_unauthed_authed_token_by_auth_code(&input.auth_code)
            .await?
            .into_iter()
            // Filter tokens created within 15 minutes using created_at and now
            .filter(|x| !x.is_activation_expired(Utc::now()))
            .collect::<Vec<_>>();

        if tokens.is_empty() {
            counter!("issue_authed_token", "state" => "failed", "reason" => "unknown").increment(1);
            return Err(anyhow::anyhow!("auth code not found"));
        }
        if tokens.len() > 1 {
            counter!("issue_authed_token", "state" => "failed", "reason" => "duplicated")
                .increment(1);
            return Err(anyhow::anyhow!("auth code is duplicated"));
        }

        let Some(authed_token_id) = self
            .0
            .get_all_authed_tokens_by_user_id(user.id)
            .await?
            .first()
            .cloned()
        else {
            return Err(anyhow::anyhow!(
                "user not found in authed tokens: auth_with_code svc"
            ));
        };

        let old_authed_token = self
            .1
            .get_authed_token_by_id(authed_token_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("authed token not found"))?;

        let token = tokens.pop().unwrap();

        let tx = self.0.begin().await?;
        let tx = self.0.bind_user_authed_token(user.id, token.id, tx).await?;
        let tx = self
            .1
            .update_authed_token_id_seed(token.id, old_authed_token.author_id_seed, tx)
            .await?;
        tx.commit().await?;

        self.1
            .activate_authed_status(&token.token, &input.user_agent, Utc::now())
            .await?;
        counter!("issue_authed_token", "state" => "success", "source" => "login").increment(1);

        Ok(AuthWithCodeUserPageServiceOutput { token: token.token })
    }
}

pub struct AuthWithCodeUserPageServiceInput {
    pub user_sid: String,
    pub auth_code: String,
    pub user_agent: String,
}

pub struct AuthWithCodeUserPageServiceOutput {
    pub token: String,
}
