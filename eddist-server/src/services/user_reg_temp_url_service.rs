use redis::{aio::ConnectionManager, AsyncCommands};
use serde::Serialize;
use sqlx::MySql;
use uuid::Uuid;

use crate::{
    domain::{
        service::bbscgi_user_reg_temp_url_service::USER_REG_TEMP_URL_LEN,
        user::user_reg_state::UserRegState,
    },
    repositories::{
        bbs_repository::BbsRepository, idp_repository::IdpRepository,
        user_repository::UserRepository,
    },
    utils::{
        redis::{user_reg_oauth2_state_key, user_reg_temp_url_register_key, user_session_key},
        TransactionRepository,
    },
};

use super::AppService;

#[derive(Clone)]
pub struct UserRegTempUrlService<I: IdpRepository, U: UserRepository, B: BbsRepository> {
    idp_repo: I,
    user_repo: U,
    bbs_repo: B,
    redis_conn: ConnectionManager,
}

impl<I: IdpRepository + Clone, U: UserRepository + Clone, B: BbsRepository + Clone>
    UserRegTempUrlService<I, U, B>
{
    pub fn new(idp_repo: I, user_repo: U, bbs_repo: B, redis_conn: ConnectionManager) -> Self {
        Self {
            idp_repo,
            user_repo,
            bbs_repo,
            redis_conn,
        }
    }
}

#[async_trait::async_trait]
impl<
        I: IdpRepository + Clone,
        U: UserRepository + Clone + TransactionRepository<MySql>,
        B: BbsRepository + Clone,
    > AppService<UserRegTempUrlServiceInput, UserRegTempUrlServiceOutput>
    for UserRegTempUrlService<I, U, B>
{
    async fn execute(
        &self,
        input: UserRegTempUrlServiceInput,
    ) -> anyhow::Result<UserRegTempUrlServiceOutput> {
        if input.temp_url_path.len() != USER_REG_TEMP_URL_LEN {
            return Ok(UserRegTempUrlServiceOutput::NotFound);
        }

        let mut redis_conn = self.redis_conn.clone();

        if let Some(user_sid) = &input.user_sid {
            if let Some(user_id) = redis_conn
                .get::<_, Option<String>>(user_session_key(user_sid))
                .await?
            {
                let Some(authed_token_id) = redis_conn
                    .get_del::<_, Option<String>>(user_reg_temp_url_register_key(
                        &input.temp_url_path,
                    ))
                    .await?
                else {
                    return Ok(UserRegTempUrlServiceOutput::NotFound);
                };

                let mut tx = self.user_repo.begin().await?;
                tx = self
                    .user_repo
                    .bind_user_authed_token(user_id.parse()?, authed_token_id.parse()?, tx)
                    .await?;
                tx.commit().await?;

                return Ok(UserRegTempUrlServiceOutput::Registered);
            }
        }

        // TODO: non-existance url
        let authed_token_id_str = redis_conn
            .get_del::<_, String>(user_reg_temp_url_register_key(&input.temp_url_path))
            .await?;

        let authed_token_id: Uuid = authed_token_id_str.parse()?;
        let authed_token_record = self
            .bbs_repo
            .get_authed_token_by_id(authed_token_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("authed token not found"))?;

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
            return Err(anyhow::anyhow!("User registration is not available"));
        }

        let state_cookie = Uuid::now_v7().to_string();
        let user_reg_state = UserRegState {
            authed_token: authed_token_id_str,
            edge_token: Some(authed_token_record.token),
            ..UserRegState::default()
        };

        redis_conn
            .set_ex::<_, _, ()>(
                user_reg_oauth2_state_key(&state_cookie),
                serde_json::to_string(&user_reg_state)?,
                60 * 3,
            )
            .await?;

        Ok(UserRegTempUrlServiceOutput::NotRegistered {
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

pub struct UserRegTempUrlServiceInput {
    pub temp_url_path: String,
    pub user_sid: Option<String>,
}

pub enum UserRegTempUrlServiceOutput {
    NotFound,
    Registered,
    NotRegistered {
        available_idps: Vec<AvailableIdp>,
        state_cookie: String,
    },
}
