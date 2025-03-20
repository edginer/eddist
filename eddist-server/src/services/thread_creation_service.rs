use std::{borrow::Cow, env, sync::OnceLock};

use chrono::Utc;
use eddist_core::{
    domain::{cap::calculate_cap_hash, client_info::ClientInfo, tinker::Tinker},
    simple_rate_limiter::RateLimiter,
};
use metrics::counter;
use redis::{aio::ConnectionManager, Cmd};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{
    domain::{
        ng_word::NgWordRestrictable,
        res::Res,
        res_core::ResCore,
        service::{
            bbscgi_auth_service::BbsCgiAuthService,
            bbscgi_user_reg_temp_url_service::{UserRegTempUrlService, UserRegUrlKind},
            board_info_service::{
                BoardInfoClientInfoResRestrictable, BoardInfoResRestrictable, BoardInfoService,
            },
            ng_word_reading_service::NgWordReadingService,
            res_creation_span_management_service::ResCreationSpanManagementService,
        },
        utils::{sanitize_base, sanitize_num_refs},
    },
    error::{BbsCgiError, NotFoundParamType},
    repositories::{
        bbs_repository::{BbsRepository, CreatingThread},
        user_repository::UserRepository,
    },
};

use super::BbsCgiService;

pub(super) static USER_CREATION_RATE_LIMIT: OnceLock<Mutex<RateLimiter>> = OnceLock::new();

#[derive(Clone)]
pub struct TheradCreationService<T: BbsRepository, U: UserRepository>(T, U, ConnectionManager);

impl<T: BbsRepository, U: UserRepository> TheradCreationService<T, U> {
    pub fn new(repo: T, user_repo: U, redis_conn: ConnectionManager) -> Self {
        Self(repo, user_repo, redis_conn)
    }
}

#[async_trait::async_trait]
impl<T: BbsRepository + Clone, U: UserRepository + Clone>
    BbsCgiService<TheradCreationServiceInput, ThreadCreationServiceOutput>
    for TheradCreationService<T, U>
{
    async fn execute(
        &self,
        input: TheradCreationServiceInput,
    ) -> Result<ThreadCreationServiceOutput, BbsCgiError> {
        let mut redis_conn = self.2.clone();
        let bbs_repo = self.0.clone();

        let (res_id, th_id) = (Uuid::now_v7(), Uuid::now_v7());
        let board_info_svc = BoardInfoService::new(self.0.clone());
        let (board, board_info) = board_info_svc
            .get_board_info_by_key(&input.board_key)
            .await?
            .ok_or_else(|| BbsCgiError::from(NotFoundParamType::Board))?;

        if board_info.read_only {
            return Err(BbsCgiError::ReadOnlyBoard);
        }

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

        let title = sanitize_thread_name(&input.title);

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

        (&res_core, &input.title as &str).validate_content_length(&board_info)?;
        client_info.validate_client_info(&board_info, true)?;

        let res = Res::new_from_thread(
            res_core,
            &input.board_key,
            created_at,
            client_info.clone(),
            input.authed_token,
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

        if input.body.starts_with("!userreg") {
            let rate_limiter = USER_CREATION_RATE_LIMIT.get_or_init(|| {
                Mutex::new(RateLimiter::new(5, std::time::Duration::from_secs(60 * 60)))
            });
            {
                let mut rate_limiter = rate_limiter.lock().await;
                if !rate_limiter.check_and_add(&authed_token.token) {
                    return Err(BbsCgiError::TooManyUserCreationAttempt);
                }
            }

            let user_reg_url_svc = UserRegTempUrlService::new(redis_conn.clone());
            return match user_reg_url_svc
                .create_userreg_temp_url(&authed_token)
                .await?
            {
                UserRegUrlKind::Registered => Err(BbsCgiError::UserAlreadyRegistered),
                UserRegUrlKind::NotRegistered(user_reg_url) => {
                    Err(BbsCgiError::UserRegTempUrl { url: user_reg_url })
                }
            };
        }

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

        let board_key = input.board_key.clone();
        let creating_th = CreatingThread {
            thread_id: th_id,
            response_id: res_id,
            title: title.to_string(),
            unix_time: unix_time as u64,
            body: res.body().to_string(),
            name: res.pretty_author_name(&board.default_name),
            mail: res.mail().to_string(),
            created_at,
            author_ch5id: res.author_id().to_string(),
            authed_token_id: authed_token.id,
            ip_addr: input.ip_addr.to_string(),
            board_id: board.id,
            metadent: res.metadent_type(),
            client_info,
        };

        let res_span_svc = ResCreationSpanManagementService::new(
            redis_conn.clone(),
            board_info.base_response_creation_span_sec as u64,
        );
        if res_span_svc
            .is_within_creation_span(&authed_token.token, unix_time as u64)
            .await
        {
            return Err(BbsCgiError::TooManyCreatingRes(
                board_info.base_response_creation_span_sec,
            ));
        };
        let ng_words = NgWordReadingService::new(self.0.clone(), redis_conn.clone())
            .get_ng_words(&input.board_key)
            .await?;
        if (&res, title.clone()).contains_ng_word(&ng_words) {
            return Err(BbsCgiError::NgWordDetected);
        }

        let authed_token_clone = authed_token.token.clone();
        let db_result = bbs_repo.create_thread(creating_th).await;
        db_result.map_err(|e| {
            if e.to_string() == "Given thread number is already exists" {
                BbsCgiError::SameTimeThreadCration
            } else {
                BbsCgiError::Other(e)
            }
        })?;
        let redis_result = tokio::spawn(async move {
            redis_conn
                .send_packed_command(&Cmd::rpush(
                    format!("thread:{}:{unix_time}", input.board_key),
                    res.get_sjis_bytes(&board.default_name, Some(&title))
                        .get_inner(),
                ))
                .await?;
            res_span_svc
                .update_last_res_creation_time(&authed_token_clone, unix_time as u64)
                .await;
            redis_conn
                .send_packed_command(&Cmd::expire(
                    format!("thread:{}:{unix_time}", input.board_key),
                    60 * 60 * 24 * 7,
                ))
                .await
        })
        .await;

        redis_result
            .map_err(|e| BbsCgiError::Other(e.into()))?
            .map_err(|e| BbsCgiError::Other(e.into()))?;

        let tinker = if let Some(tinker) = input.tinker {
            if tinker.authed_token() != authed_token.token {
                Tinker::new(authed_token.token, created_at)
            } else {
                tinker
            }
        } else {
            Tinker::new(authed_token.token, created_at)
        }
        .action_on_create_thread(created_at);

        let _ = bbs_repo
            .update_authed_token_last_wrote(authed_token.id, created_at)
            .await;

        counter!("response_creation", "board_key" => board_key.clone()).increment(1);
        counter!("thread_creation", "board_key" => board_key.clone()).increment(1);

        Ok(ThreadCreationServiceOutput { tinker })
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

pub struct ThreadCreationServiceOutput {
    pub tinker: Tinker,
}

pub fn sanitize_thread_name(name: &str) -> String {
    sanitize_num_refs(&sanitize_base(name, false))
}
