use std::{borrow::Cow, env};

use anyhow::anyhow;
use chrono::Utc;
use eddist_core::domain::{
    client_info::ClientInfo,
    pubsub_repository::{CreatingRes, PubSubItem},
    tinker::Tinker,
};
use metrics::counter;
use redis::{aio::ConnectionManager, Cmd, Value};
use tracing::error_span;
use uuid::Uuid;

use crate::{
    domain::{
        cap::calculate_cap_hash,
        ng_word::NgWordRestrictable,
        res::Res,
        res_core::ResCore,
        service::{
            bbscgi_auth_service::BbsCgiAuthService,
            board_info_service::{
                BoardInfoClientInfoResRestrictable, BoardInfoResRestrictable, BoardInfoService,
            },
            ng_word_reading_service::NgWordReadingService,
            res_creation_span_management_service::ResCreationSpanManagementService,
        },
    },
    error::{BbsCgiError, NotFoundParamType},
    repositories::{bbs_pubsub_repository::PubRepository, bbs_repository::BbsRepository},
};

use super::BbsCgiService;

#[derive(Clone)]
pub struct ResCreationService<T: BbsRepository, P: PubRepository>(T, ConnectionManager, P);

impl<T: BbsRepository, P: PubRepository> ResCreationService<T, P> {
    pub fn new(repo: T, redis_conn: ConnectionManager, pub_repo: P) -> Self {
        Self(repo, redis_conn, pub_repo)
    }
}

#[async_trait::async_trait]
impl<T: BbsRepository + Clone, P: PubRepository>
    BbsCgiService<ResCreationServiceInput, ResCreationServiceOutput> for ResCreationService<T, P>
{
    async fn execute(
        &self,
        input: ResCreationServiceInput,
    ) -> Result<ResCreationServiceOutput, BbsCgiError> {
        let mut redis_conn = self.1.clone();
        let bbs_repo = self.0.clone();
        let pub_repo = self.2.clone();

        let res_id = Uuid::now_v7();
        let board_info_svc = BoardInfoService::new(self.0.clone());
        let (board, board_info) = board_info_svc
            .get_board_info_by_key(&input.board_key)
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

        let res_core = ResCore {
            from: &input.name,
            mail: &input.mail,
            body: Cow::Borrowed(&input.body),
        };
        let client_info = ClientInfo {
            user_agent: input.user_agent.clone(),
            asn_num: input.asn_num,
            ip_addr: input.ip_addr.clone(),
            tinker: input.tinker.as_ref().map(|x| Box::new(x.clone())),
        };

        res_core.validate_content_length(&board_info)?;
        client_info.validate_client_info(&board_info, false)?;

        let res = Res::new_from_res(
            res_core,
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

        let res_span_svc = ResCreationSpanManagementService::new(
            redis_conn.clone(),
            board_info.base_response_creation_span_sec as u64,
        );
        if res_span_svc
            .is_within_creation_span(&authed_token.token, created_at.timestamp() as u64)
            .await
        {
            return Err(BbsCgiError::TooManyCreatingRes(
                board_info.base_response_creation_span_sec,
            ));
        };

        // Check thread:{board_key}:{thread_number} exists. If not, does not rpush to the list but still creates the response in the database.
        let is_exists = matches!(
            redis_conn
                .send_packed_command(&Cmd::exists(format!(
                    "thread:{}:{}",
                    input.board_key, input.thread_number
                )))
                .await
                .map_err(|e| BbsCgiError::Other(e.into()))?,
            Value::Int(i) if i > 0
        );
        let order = if is_exists {
            let Value::Int(order) = redis_conn
                .send_packed_command(&Cmd::rpush(
                    format!("thread:{}:{}", input.board_key, input.thread_number),
                    res.get_sjis_bytes(&board.default_name, None).get_inner(),
                ))
                .await
                .map_err(|e| BbsCgiError::Other(e.into()))?
            else {
                return Err(BbsCgiError::Other(anyhow!(
                    "failed to parse redis response"
                )));
            };
            order as i32
        } else {
            // Sort by order, and then by id (uuidv7), thus the order of non-cache-existence response is over 1000.
            10000
        };

        res_span_svc
            .update_last_res_creation_time(&authed_token.token, created_at.timestamp() as u64)
            .await;

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
            is_sage: res.is_sage(),
        };
        tokio::spawn(async move {
            if let Err(e) = bbs_repo.create_response(cres.clone()).await {
                error_span!("failed to create response in database",
                    error = %e
                );
                pub_repo
                    .publish(PubSubItem::CreatingRes(Box::new(cres)))
                    .await
                    .unwrap();
            }
        });

        let tinker = if let Some(tinker) = input.tinker {
            tinker
        } else {
            Tinker::new(authed_token.token, created_at)
        }
        .action_on_write(created_at);

        counter!("response_creation", "board_key" => input.board_key).increment(1);

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
