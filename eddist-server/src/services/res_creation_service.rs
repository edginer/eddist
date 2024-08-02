use chrono::Utc;
use redis::{aio::MultiplexedConnection, Cmd, Value};
use uuid::Uuid;

use crate::{
    domain::{
        authed_token::AuthedToken, client_info::ClientInfo, res::Res, res_core::ResCore,
        tinker::Tinker,
    },
    error::{BbsCgiError, NotFoundParamType},
    repositories::bbs_repository::{BbsRepository, CreatingAuthedToken, CreatingRes},
};

use super::BbsCgiService;

#[derive(Debug, Clone)]
pub struct ResCreationService<T: BbsRepository>(T, MultiplexedConnection);

impl<T: BbsRepository> ResCreationService<T> {
    pub fn new(repo: T, redis_conn: MultiplexedConnection) -> Self {
        Self(repo, redis_conn)
    }
}

#[async_trait::async_trait]
impl<T: BbsRepository + Clone> BbsCgiService<ResCreationServiceInput, ResCreationServiceOutput>
    for ResCreationService<T>
{
    async fn execute(
        &self,
        input: ResCreationServiceInput,
    ) -> Result<ResCreationServiceOutput, BbsCgiError> {
        let mut redis_conn = self.1.clone();
        let bbs_repo = self.0.clone();

        let res_id = Uuid::now_v7();
        let board = self
            .0
            .get_board_info(&input.board_key)
            .await?
            .ok_or_else(|| BbsCgiError::from(NotFoundParamType::Board))?;
        let created_at = Utc::now();

        let Some(th) = self
            .0
            .get_thread_by_board_key_and_thread_number(&input.board_key, input.thread_number)
            .await
            .map_err(BbsCgiError::Other)?
        else {
            return Err(BbsCgiError::from(NotFoundParamType::Thread));
        };

        if !th.active {
            return Err(BbsCgiError::InactiveThread);
        }

        let res = Res::new_from_res(
            ResCore {
                from: &input.name,
                mail: &input.mail,
                body: input.body.clone(),
            },
            &input.board_key,
            created_at,
            (&th.metadent as &str).into(),
            ClientInfo {
                user_agent: input.user_agent.clone(),
                asn_num: input.asn_num,
                ip_addr: input.ip_addr.clone(),
                tinker: input.tinker.as_ref().map(|x| Box::new(x.clone())),
            },
            input.authed_token_cookie,
            false,
        );

        let Some(authed_token) = res.authed_token() else {
            let authed_token = AuthedToken::new(input.ip_addr, input.user_agent);
            self.0
                .create_authed_token(CreatingAuthedToken {
                    token: authed_token.token.clone(),
                    writing_ua: authed_token.writing_ua,
                    origin_ip: authed_token.origin_ip,
                    created_at,
                    auth_code: authed_token.auth_code.clone(),
                    id: authed_token.id,
                })
                .await?;

            return Err(BbsCgiError::Unauthenticated {
                auth_code: authed_token.auth_code,
                base_url: "http://localhost:8080".to_string(),
                auth_token: authed_token.token,
            });
        };

        let authed_token = self
            .0
            .get_authed_token(authed_token)
            .await
            .map_err(BbsCgiError::Other)?
            .ok_or_else(|| BbsCgiError::InvalidAuthedToken)?;

        if !authed_token.validity {
            return if authed_token.authed_at.is_some() {
                Err(BbsCgiError::RevokedAuthedToken)
            } else if authed_token.is_activation_expired(Utc::now()) {
                let authed_token = AuthedToken::new(input.ip_addr, input.user_agent);
                self.0
                    .create_authed_token(CreatingAuthedToken {
                        token: authed_token.token.clone(),
                        writing_ua: authed_token.writing_ua,
                        origin_ip: authed_token.origin_ip,
                        created_at,
                        auth_code: authed_token.auth_code.clone(),
                        id: authed_token.id,
                    })
                    .await?;

                return Err(BbsCgiError::Unauthenticated {
                    auth_code: authed_token.auth_code,
                    base_url: "http://localhost:8080".to_string(),
                    auth_token: authed_token.token,
                });
            } else {
                Err(BbsCgiError::Unauthenticated {
                    auth_code: authed_token.auth_code,
                    base_url: "http://localhost:8080".to_string(),
                    auth_token: authed_token.token,
                })
            };
        }

        if let Value::Int(order) = redis_conn
            .send_packed_command(&Cmd::rpush(
                format!("thread/{}/{}", input.board_key, input.thread_number),
                res.get_sjis_bytes(&board.default_name, None).get_inner(),
            ))
            .await
            .map_err(|e| BbsCgiError::Other(e.into()))?
        {
            let cres = CreatingRes {
                id: res_id,
                created_at,
                body: res.body().to_string(),
                name: res.author_name().to_string(),
                mail: res.mail().to_string(),
                author_ch5id: res.author_id().to_string(),
                authed_token_id: authed_token.id,
                ip_addr: input.ip_addr,
                thread_id: th.id,
                board_id: th.board_id,
                res_order: order as i32,
            };
            tokio::spawn(async move { bbs_repo.create_response(cres).await });
        }
        let tinker = if let Some(tinker) = input.tinker {
            tinker
        } else {
            Tinker::new(authed_token.token, created_at)
        }
        .action_on_write(created_at);

        Ok(ResCreationServiceOutput { tinker })
    }
}

pub struct ResCreationServiceInput {
    pub board_key: String,
    pub thread_number: u64,
    pub authed_token_cookie: Option<String>,
    pub name: String,
    pub mail: String,
    pub body: String,
    pub tinker: Option<Tinker>,
    pub ip_addr: String,
    pub user_agent: String,
    pub asn_num: u32,
}

pub struct ResCreationServiceOutput {
    pub tinker: Tinker,
}
