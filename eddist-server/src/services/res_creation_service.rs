use std::borrow::Cow;

use anyhow::anyhow;
use chrono::Utc;
use eddist_core::{
    domain::{
        client_info::ClientInfo,
        ip_addr::ReducedIpAddr,
        pubsub_repository::{CreatingRes, PubSubItem},
        tinker::Tinker,
    },
    utils::is_res_pub_enabled,
};
use metrics::counter;
use redis::{Cmd, Value, aio::ConnectionManager};
use tracing::error_span;
use uuid::Uuid;

use crate::{
    domain::{
        ng_word::NgWordRestrictable,
        res::Res,
        res_core::ResCore,
        service::{
            bbscgi_auth_service::BbsCgiAuthService,
            board_info_service::{
                BoardInfoClientInfoResRestrictable, BoardInfoResRestrictable, BoardInfoService,
            },
            email_auth_restriction_service::EmailAuthRestrictionService,
            ng_word_reading_service::NgWordReadingService,
            res_creation_span_management_service::ResCreationSpanManagementService,
        },
        utils::count_anchors,
    },
    error::{BbsCgiError, NotFoundParamType},
    repositories::{
        bbs_pubsub_repository::{CreationEventRepository, PubRepository},
        bbs_repository::BbsRepository,
        user_repository::UserRepository,
    },
};
use eddist_core::redis_keys::thread_cache_key;

use super::{
    BbsCgiService, openai_moderation_service,
    server_settings_cache::{ServerSettingKey, get_server_setting_bool},
    validation::{check_userreg, resolve_cap_name},
};

#[derive(Clone)]
pub struct ResCreationService<
    T: BbsRepository,
    U: UserRepository,
    P: PubRepository,
    E: CreationEventRepository,
>(T, U, ConnectionManager, P, E);

impl<T: BbsRepository, U: UserRepository, P: PubRepository, E: CreationEventRepository>
    ResCreationService<T, U, P, E>
{
    pub fn new(
        repo: T,
        user_repo: U,
        redis_conn: ConnectionManager,
        pub_repo: P,
        event_repo: E,
    ) -> Self {
        Self(repo, user_repo, redis_conn, pub_repo, event_repo)
    }
}

#[async_trait::async_trait]
impl<
    T: BbsRepository + Clone,
    U: UserRepository + Clone,
    P: PubRepository,
    E: CreationEventRepository,
> BbsCgiService<ResCreationServiceInput, ResCreationServiceOutput>
    for ResCreationService<T, U, P, E>
{
    async fn execute(
        &self,
        input: ResCreationServiceInput,
    ) -> Result<ResCreationServiceOutput, BbsCgiError> {
        let mut redis_conn = self.2.clone();
        let bbs_repo = self.0.clone();
        let pub_repo = self.3.clone();

        let res_id = Uuid::now_v7();
        let board_info_svc = BoardInfoService::new(self.0.clone());
        let (board, board_info) = board_info_svc
            .get_board_info_by_key(&input.board_key)
            .await?
            .ok_or_else(|| BbsCgiError::from(NotFoundParamType::Board))?;

        if board_info.read_only {
            return Err(BbsCgiError::ReadOnlyBoard);
        }

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

        let metadent_type = board_info
            .force_metadent_type
            .as_deref()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| (&th.metadent as &str).into());

        let res = Res::new_from_res(
            res_core,
            &input.board_key,
            created_at,
            metadent_type,
            client_info.clone(),
            input.authed_token_cookie,
            false,
        );

        let auth_service =
            BbsCgiAuthService::new(self.0.clone(), redis_conn.clone(), self.4.clone());
        let authed_token = auth_service
            .check_validity(
                res.authed_token().map(|x| x.as_str()),
                input.ip_addr.clone(),
                input.user_agent.clone(),
                input.asn_num as i32,
                created_at,
                input.require_user_registration,
            )
            .await?;

        let email_auth_service = EmailAuthRestrictionService::new(redis_conn.clone());
        email_auth_service
            .check_and_enforce_restriction(
                res.is_email_authed(),
                &input.user_agent,
                &authed_token.token,
                &input.ip_addr,
            )
            .await?;

        if let Some(err) = check_userreg(&input.body, &authed_token, redis_conn.clone()).await? {
            return Err(err);
        }

        let cap_name = resolve_cap_name(&self.0, &res, &input.board_key).await?;
        let mut res = res.set_author_id(&authed_token, cap_name);

        // Restrict the image posting below internal_level 2
        if let Some(tinker) = &input.tinker {
            if tinker.internal_level() < 2 && !res.get_all_images().is_empty() {
                return Err(BbsCgiError::ImageUrlBelowLv2);
            }
        } else if !res.get_all_images().is_empty() {
            // Does not allow image URL
            return Err(BbsCgiError::ImageUrlBelowLv2);
        }

        // Restrict the anchor count below internal_level 2
        const MAX_ANCHORS_BELOW_LV2: usize = 3;
        let needs_anchor_check = if let Some(tinker) = &input.tinker {
            tinker.internal_level() < 2
        } else {
            true
        };

        if needs_anchor_check && count_anchors(&input.body) > MAX_ANCHORS_BELOW_LV2 {
            return Err(BbsCgiError::NgWordDetected);
        }

        let ng_words = NgWordReadingService::new(self.0.clone(), redis_conn.clone())
            .get_ng_words(&input.board_key)
            .await?;
        if res.contains_ng_word(&ng_words) {
            return Err(BbsCgiError::NgWordDetected);
        }

        // Determine response creation span and tinker level based on tinker
        // Level 1 users: 30 seconds, Level 2+ users: 5 seconds (base_response_creation_span_sec)
        let (_, response_span_sec) = if let Some(tinker) = &input.tinker {
            let level = tinker.level();
            let span = if level < 2 {
                30_u64
            } else {
                board_info.base_response_creation_span_sec as u64
            };
            (level, span)
        } else {
            // No tinker (not authenticated) - use level 1 and 30 seconds restriction
            (1, 30_u64)
        };

        let res_span_svc = ResCreationSpanManagementService::new(
            redis_conn.clone(),
            response_span_sec,
            board_info.base_thread_creation_span_sec as u64, // ignorable
        );
        if res_span_svc
            .is_within_creation_span(
                &authed_token.reduced_ip.to_string(),
                &ReducedIpAddr::from(input.ip_addr.clone()).to_string(),
                created_at.timestamp() as u64,
            )
            .await
        {
            // Get the actual wait time considering all restrictions (1-hour restriction, penalties, etc.)
            let wait_time = res_span_svc
                .get_actual_wait_time_for_auth_ip(&authed_token.reduced_ip.to_string())
                .await;

            return Err(BbsCgiError::ResCreationSpanRestriction {
                wait_sec: wait_time as u32,
            });
        };

        // RPUSHX appends only if the thread cache key exists, so the check and push are
        // atomic (no EXISTS/RPUSH TTL race). Returns 0 if absent; we still persist to the DB.
        let Value::Int(order) = redis_conn
            .send_packed_command(&Cmd::rpush_exists(
                thread_cache_key(&input.board_key, input.thread_number),
                res.get_sjis_bytes(&board.default_name, None).get_inner(),
            ))
            .await
            .map_err(|e| BbsCgiError::Other(e.into()))?
        else {
            return Err(BbsCgiError::Other(anyhow!(
                "failed to parse redis response"
            )));
        };
        let order = if order > 0 {
            order as i32
        } else {
            // Sort by order, and then by id (uuidv7), thus the order of non-cache-existence response is over 1000.
            10000
        };

        res_span_svc
            .update_last_res_creation_time(
                &authed_token.reduced_ip.to_string(),
                &ReducedIpAddr::from(input.ip_addr.clone()).to_string(),
                created_at.timestamp() as u64,
            )
            .await;

        let cres = CreatingRes {
            id: res_id,
            created_at,
            body: res.body().to_string(),
            name: res.pretty_author_name(&board.default_name),
            mail: res.mail().to_string(),
            author_ch5id: res.author_id().to_string(),
            authed_token_id: authed_token.id,
            ip_addr: input.ip_addr,
            thread_id: th.id,
            board_id: th.board_id,
            client_info,
            res_order: order as i32,
            is_sage: res.is_sage(),
            moderation_result: None,
        };

        let event_repo = self.4.clone();

        tokio::spawn(async move {
            if let Err(e) = bbs_repo.create_response(cres.clone()).await {
                error_span!("failed to create response in database",
                    error = %e
                );
                pub_repo
                    .publish(PubSubItem::CreatingRes(Box::new(cres.clone())))
                    .await
                    .unwrap();
            }

            if is_res_pub_enabled() {
                let moderation_result =
                    if get_server_setting_bool(ServerSettingKey::AiModerationOnRes).await {
                        openai_moderation_service::moderate(&cres.body).await
                    } else {
                        None
                    };
                let mut cres = cres;
                cres.moderation_result = moderation_result;
                let _ = event_repo.publish_res_created(cres).await;
            }

            let _ = bbs_repo
                .update_authed_token_last_wrote(authed_token.id, created_at)
                .await;
        });

        let tinker = if let Some(tinker) = input.tinker {
            if tinker.authed_token() != authed_token.token {
                Tinker::new(authed_token.token, created_at)
            } else {
                tinker
            }
        } else {
            Tinker::new(authed_token.token, created_at)
        }
        .action_on_write(created_at);

        counter!("response_creation", "board_key" => input.board_key).increment(1);

        let res_order = if order <= 2000 { Some(order) } else { None };

        Ok(ResCreationServiceOutput {
            tinker,
            res_order,
            authed_token_id: authed_token.id,
            is_authed_token_bound: authed_token.registered_user_id.is_some(),
        })
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
    pub require_user_registration: bool,
}

pub struct ResCreationServiceOutput {
    pub tinker: Tinker,
    pub res_order: Option<i32>,
    pub authed_token_id: Uuid,
    pub is_authed_token_bound: bool,
}
