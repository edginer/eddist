use std::{borrow::Cow, env};

use chrono::Utc;
use eddist_core::domain::{client_info::ClientInfo, tinker::Tinker};
use redis::{aio::MultiplexedConnection, Cmd, Value};
use uuid::Uuid;

use crate::{
    domain::{
        cap::calculate_cap_hash,
        ng_word::NgWordRestrictable,
        res::Res,
        res_core::ResCore,
        service::{
            bbscgi_auth_service::BbsCgiAuthService, ng_word_reading_service::NgWordReadingService,
        },
    },
    error::{BbsCgiError, NotFoundParamType},
    repositories::bbs_repository::{BbsRepository, CreatingRes},
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

        let client_info = ClientInfo {
            user_agent: input.user_agent.clone(),
            asn_num: input.asn_num,
            ip_addr: input.ip_addr.clone(),
            tinker: input.tinker.as_ref().map(|x| Box::new(x.clone())),
        };
        let res = Res::new_from_res(
            ResCore {
                from: &input.name,
                mail: &input.mail,
                body: Cow::Borrowed(&input.body),
            },
            &input.board_key,
            created_at,
            (&th.metadent as &str).into(),
            client_info.clone(),
            input.authed_token_cookie,
            false,
        );

        let auth_service = BbsCgiAuthService::new(self.0.clone());
        let authed_token = auth_service
            .check_validity(
                res.authed_token().map(|x| x.as_str()),
                input.ip_addr.clone(),
                input.user_agent,
                created_at,
            )
            .await?;
        let cap_name = if let Some(cap) = res.cap() {
            let hash = calculate_cap_hash(cap.get(), &env::var("TINKER_SECRET").unwrap());
            self.0
                .get_cap_by_board_key(&hash, &input.board_key)
                .await?
                .map(|x| x.name)
        } else {
            None
        };
        let res = res.set_author_id(&authed_token, cap_name);

        let ng_words = NgWordReadingService::new(self.0.clone(), redis_conn.clone())
            .get_ng_words(&input.board_key)
            .await?;
        if res.contains_ng_word(&ng_words) {
            return Err(BbsCgiError::NgWordDetected);
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
                client_info,
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
