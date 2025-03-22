use redis::{aio::ConnectionManager, AsyncCommands};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    domain::{
        service::bbscgi_user_reg_temp_url_service::USER_REG_TEMP_URL_LEN,
        user::user_reg_state::UserRegState,
    },
    repositories::idp_repository::IdpRepository,
};

use super::AppService;

#[derive(Clone)]
pub struct UserRegTempUrlService<I: IdpRepository> {
    idp_repo: I,
    redis_conn: ConnectionManager,
}

impl<I: IdpRepository + Clone> UserRegTempUrlService<I> {
    pub fn new(idp_repo: I, redis_conn: ConnectionManager) -> Self {
        Self {
            idp_repo,
            redis_conn,
        }
    }
}

#[async_trait::async_trait]
impl<I: IdpRepository + Clone> AppService<UserRegTempUrlServiceInput, UserRegTempUrlServiceOutput>
    for UserRegTempUrlService<I>
{
    async fn execute(
        &self,
        input: UserRegTempUrlServiceInput,
    ) -> anyhow::Result<UserRegTempUrlServiceOutput> {
        if input.temp_url_path.len() != USER_REG_TEMP_URL_LEN {
            return Ok(UserRegTempUrlServiceOutput::NotFound);
        }

        let mut redis_conn = self.redis_conn.clone();

        if let Some(user_cookie) = &input.user_cookie {
            if redis_conn
                .exists::<_, bool>(format!("user:session:{user_cookie}"))
                .await?
            {
                redis_conn
                    .del::<_, ()>(format!("userreg:tempurl:register:{}", input.temp_url_path))
                    .await?;
                return Ok(UserRegTempUrlServiceOutput::Registered);
            }
        }

        // TODO: non-existance url
        let authed_token = redis_conn
            .get_del::<_, String>(format!("userreg:tempurl:register:{}", input.temp_url_path))
            .await?;

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
            authed_token,
            ..UserRegState::default()
        };

        redis_conn
            .set_ex::<_, _, ()>(
                format!("userreg:oauth2:state:{}", state_cookie),
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
    pub user_cookie: Option<String>,
}

pub enum UserRegTempUrlServiceOutput {
    NotFound,
    Registered,
    NotRegistered {
        available_idps: Vec<AvailableIdp>,
        state_cookie: String,
    },
}
