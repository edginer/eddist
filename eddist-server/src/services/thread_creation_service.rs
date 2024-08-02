use chrono::Utc;
use redis::{aio::MultiplexedConnection, Cmd};
use tokio::join;
use uuid::Uuid;

use crate::{
    domain::{
        authed_token::AuthedToken, client_info::ClientInfo, metadent::MetadentType, res::Res,
        res_core::ResCore, tinker::Tinker,
    },
    error::{BbsCgiError, NotFoundParamType},
    repositories::bbs_repository::{BbsRepository, CreatingAuthedToken, CreatingThread},
};

use super::BbsCgiService;

#[derive(Debug, Clone)]
pub struct TheradCreationService<T: BbsRepository>(T, MultiplexedConnection);

impl<T: BbsRepository> TheradCreationService<T> {
    pub fn new(repo: T, redis_conn: MultiplexedConnection) -> Self {
        Self(repo, redis_conn)
    }
}

#[async_trait::async_trait]
impl<T: BbsRepository + Clone>
    BbsCgiService<TheradCreationServiceInput, TheradCreationServiceOutput>
    for TheradCreationService<T>
{
    async fn execute(
        &self,
        input: TheradCreationServiceInput,
    ) -> Result<TheradCreationServiceOutput, BbsCgiError> {
        let mut redis_conn = self.1.clone();
        let bbs_repo = self.0.clone();

        let (res_id, th_id) = (Uuid::now_v7(), Uuid::now_v7());
        let board = self
            .0
            .get_board_info(&input.board_key)
            .await
            .map_err(BbsCgiError::Other)?
            .ok_or_else(|| BbsCgiError::NotFound(NotFoundParamType::Board))?;
        let created_at = Utc::now();
        let unix_time = created_at.timestamp();

        if self
            .0
            .get_thread_by_board_key_and_thread_number(&input.board_key, unix_time as u64)
            .await
            .unwrap()
            .is_some()
        {
            return Err(BbsCgiError::SameTimeThreadCration);
        }

        let title = input.title.clone();

        let res = Res::new_from_thread(
            ResCore {
                from: &input.name,
                mail: &input.mail,
                body: input.body.clone(),
            },
            &input.board_key,
            created_at,
            ClientInfo {
                user_agent: input.user_agent.clone(),
                asn_num: input.asn_num,
                ip_addr: input.ip_addr.clone(),
                tinker: None,
            },
            input.authed_token,
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
                    auth_code: authed_token.auth_code.to_string(),
                    id: authed_token.id,
                })
                .await?;

            return Err(BbsCgiError::Unauthenticated {
                auth_code: authed_token.auth_code.to_string(),
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

        // NOTE: duplication code from res_creation_service.rs
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

        let creating_th = CreatingThread {
            thread_id: th_id,
            response_id: res_id,
            title: input.title.to_string(),
            unix_time: unix_time as u64,
            body: input.body.to_string(),
            name: input.name.to_string(),
            mail: input.mail.to_string(),
            created_at,
            author_ch5id: res.author_id().to_string(),
            authed_token_id: authed_token.id,
            ip_addr: input.ip_addr.to_string(),
            board_id: board.id,
            metadent: MetadentType::None,
        };

        let db_req = tokio::spawn(async move { bbs_repo.create_thread(creating_th).await });
        let redis_req = tokio::spawn(async move {
            redis_conn
                .send_packed_command(&Cmd::rpush(
                    format!("thread/{}/{unix_time}", input.board_key),
                    res.get_sjis_bytes(&board.default_name, Some(&title))
                        .get_inner(),
                ))
                .await
        });

        let (db_result, redis_result) = join!(db_req, redis_req);

        db_result
            .map_err(|e| BbsCgiError::Other(e.into()))?
            .map_err(|e| {
                if e.to_string() == "Given thread number is already exists" {
                    BbsCgiError::SameTimeThreadCration
                } else {
                    BbsCgiError::Other(e)
                }
            })?;
        redis_result
            .map_err(|e| BbsCgiError::Other(e.into()))?
            .map_err(|e| BbsCgiError::Other(e.into()))?;

        let tinker = if let Some(tinker) = input.tinker {
            tinker
        } else {
            Tinker::new(authed_token.token, created_at)
        }
        .action_on_create_thread(created_at);

        Ok(TheradCreationServiceOutput { tinker })
    }
}

pub struct TheradCreationServiceInput {
    pub board_key: String,
    pub title: String,
    pub authed_token: Option<String>,
    pub name: String,
    pub mail: String,
    pub body: String,
    pub tinker: Option<Tinker>,
    pub ip_addr: String,
    pub user_agent: String,
    pub asn_num: u32,
}

pub struct TheradCreationServiceOutput {
    pub tinker: Tinker,
}
