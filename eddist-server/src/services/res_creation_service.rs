use std::{borrow::Cow, sync::Arc};

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
use tracing::error;
use uuid::Uuid;

use crate::{
    context::AppContext,
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
        },
        utils::count_anchors,
    },
    error::{BbsCgiError, NotFoundParamType},
    repositories::{
        bbs_pubsub_repository::{CreationEventRepository, PubRepository, record_event_publish},
        bbs_repository::{BbsRepository, ResponseRepository},
    },
    stores::PostingStores,
};

use super::{
    BbsCgiService, openai_moderation_service,
    server_settings_cache::{ServerSettingKey, ServerSettingsSnapshot},
    validation::{check_userreg, resolve_cap_name},
};

#[derive(Clone)]
pub struct ResCreationService<T: BbsRepository, P: PubRepository, E: CreationEventRepository> {
    bbs_repo: T,
    stores: PostingStores,
    pub_repo: P,
    event_repo: E,
    ctx: AppContext,
}

impl<T: BbsRepository, P: PubRepository, E: CreationEventRepository> ResCreationService<T, P, E> {
    pub fn new(
        bbs_repo: T,
        stores: PostingStores,
        pub_repo: P,
        event_repo: E,
        ctx: AppContext,
    ) -> Self {
        Self {
            bbs_repo,
            stores,
            pub_repo,
            event_repo,
            ctx,
        }
    }
}

#[async_trait::async_trait]
impl<T: BbsRepository + Clone, P: PubRepository, E: CreationEventRepository>
    BbsCgiService<ResCreationServiceInput, ResCreationServiceOutput>
    for ResCreationService<T, P, E>
{
    async fn execute(
        &self,
        input: ResCreationServiceInput,
    ) -> Result<ResCreationServiceOutput, BbsCgiError> {
        let bbs_repo = self.bbs_repo.clone();
        let pub_repo = self.pub_repo.clone();

        let res_id = Uuid::now_v7();
        let board_info_svc =
            BoardInfoService::new(self.bbs_repo.clone(), self.ctx.board_info.clone());
        let (board, board_info) = board_info_svc
            .get_board_info_by_key(&input.board_key)
            .await?
            .ok_or_else(|| BbsCgiError::from(NotFoundParamType::Board))?;

        if board_info.read_only {
            return Err(BbsCgiError::ReadOnlyBoard);
        }

        let created_at = Utc::now();

        let Some(th) = self
            .bbs_repo
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
        client_info.validate_client_info(
            &board_info,
            false,
            self.ctx.config.restrict_thread_creation_on_no_tinker,
        )?;

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

        let auth_service = BbsCgiAuthService::new(
            self.bbs_repo.clone(),
            self.stores.auth_state.clone(),
            self.event_repo.clone(),
            self.ctx.config.clone(),
            self.ctx.user_creation_rate_limit.clone(),
            self.ctx.background_tasks.clone(),
        );
        let authed_token = auth_service
            .check_validity(
                res.authed_token().map(|x| x.as_str()),
                input.ip_addr.clone(),
                input.user_agent.clone(),
                input.asn_num as i32,
                created_at,
                &input.settings,
            )
            .await?;

        let email_auth_service = EmailAuthRestrictionService::new(self.stores.auth_state.clone());
        email_auth_service
            .check_and_enforce_restriction(
                res.is_email_authed(),
                &input.user_agent,
                &authed_token.token,
                &input.ip_addr,
            )
            .await?;

        if let Some(err) = check_userreg(
            &input.body,
            &authed_token,
            self.stores.auth_state.as_ref(),
            &input.settings,
            &self.ctx.config.base_url,
            &self.ctx.user_creation_rate_limit,
        )
        .await?
        {
            return Err(err);
        }

        let cap_name = resolve_cap_name(
            &self.bbs_repo,
            &res,
            &input.board_key,
            &self.ctx.config.tinker_secret,
        )
        .await?;
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

        let ng_words = self.stores.ng_words.get_ng_words(&input.board_key).await?;
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

        let auth_ip = authed_token.reduced_ip.to_string();
        let ip = ReducedIpAddr::from(input.ip_addr.clone()).to_string();
        let now = created_at.timestamp() as u64;
        let rate_limit = &self.stores.rate_limit;
        if rate_limit
            .is_res_restricted(response_span_sec, &auth_ip, &ip, now)
            .await
        {
            return Err(BbsCgiError::ResCreationSpanRestriction {
                wait_sec: rate_limit.res_wait_secs(response_span_sec, &auth_ip).await as u32,
            });
        };

        let cached_len = self
            .stores
            .dat_cache
            .append_if_cached(
                &input.board_key,
                input.thread_number,
                res.get_sjis_bytes(&board.default_name, None).get_inner(),
            )
            .await?;
        // Sort by order, and then by id (uuidv7), thus the order of non-cache-existence response is over 1000.
        let order = cached_len.map_or(10000, |len| len as i32);

        rate_limit
            .record_res(response_span_sec, &auth_ip, &ip, now)
            .await?;

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

        let event_repo = self.event_repo.clone();
        let settings = input.settings.clone();

        self.ctx
            .background_tasks
            .spawn_detached("res_persistence", async move {
                persist_response(&bbs_repo, &pub_repo, &cres).await;

                if is_res_pub_enabled() {
                    let moderation_result =
                        if settings.is_enabled(ServerSettingKey::AiModerationOnRes) {
                            openai_moderation_service::moderate(&settings, &cres.body).await
                        } else {
                            None
                        };
                    let mut cres = cres;
                    cres.moderation_result = moderation_result;
                    record_event_publish("res_created", event_repo.publish_res_created(cres).await);
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

async fn persist_response<R: ResponseRepository, P: PubRepository>(
    repo: &R,
    publisher: &P,
    res: &CreatingRes,
) {
    match repo.create_response(res.clone()).await {
        Ok(()) => {
            counter!("response_persistence", "result" => "db_success").increment(1);
        }
        Err(e) => {
            counter!("response_persistence", "result" => "db_failure").increment(1);
            error!(
                error = %e,
                res_id = %res.id,
                thread_id = %res.thread_id,
                "failed to create response in database"
            );
            match publisher
                .publish(PubSubItem::CreatingRes(Box::new(res.clone())))
                .await
            {
                Ok(()) => {
                    counter!("response_persistence", "result" => "fallback_success").increment(1);
                }
                Err(e) => {
                    counter!("response_persistence", "result" => "fallback_failure").increment(1);
                    error!(
                        error = %e,
                        res_id = %res.id,
                        thread_id = %res.thread_id,
                        "failed to publish response persistence fallback"
                    );
                }
            }
        }
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
    pub settings: Arc<ServerSettingsSnapshot>,
}

pub struct ResCreationServiceOutput {
    pub tinker: Tinker,
    pub res_order: Option<i32>,
    pub authed_token_id: Uuid,
    pub is_authed_token_bound: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use eddist_core::domain::res::ResView;
    use eddist_core::test_utils::LogCapture;
    use std::sync::{Arc, Mutex};
    use tracing::instrument::WithSubscriber;

    struct Repository {
        fail: bool,
    }

    #[async_trait::async_trait]
    impl ResponseRepository for Repository {
        async fn get_responses(&self, _: Uuid) -> anyhow::Result<Vec<ResView>> {
            unreachable!()
        }

        async fn create_response(&self, _: CreatingRes) -> anyhow::Result<()> {
            anyhow::ensure!(!self.fail, "database unavailable");
            Ok(())
        }
    }

    #[derive(Clone)]
    struct Publisher {
        fail: bool,
        items: Arc<Mutex<Vec<PubSubItem>>>,
    }

    #[async_trait::async_trait]
    impl PubRepository for Publisher {
        async fn publish(&self, item: PubSubItem) -> anyhow::Result<()> {
            self.items.lock().unwrap().push(item);
            anyhow::ensure!(!self.fail, "redis unavailable");
            Ok(())
        }
    }

    #[tokio::test]
    async fn persistence_failures_are_logged_and_fallback_does_not_panic() {
        let res = CreatingRes {
            id: Uuid::now_v7(),
            created_at: Utc::now(),
            body: "private-response-body".into(),
            name: "name".into(),
            mail: String::new(),
            author_ch5id: "author".into(),
            authed_token_id: Uuid::now_v7(),
            ip_addr: "192.0.2.99".into(),
            thread_id: Uuid::now_v7(),
            board_id: Uuid::now_v7(),
            client_info: ClientInfo::default(),
            res_order: 1,
            is_sage: false,
            moderation_result: None,
        };
        for (db_fail, fallback_fail) in [(false, false), (true, false), (true, true)] {
            let repo = Repository { fail: db_fail };
            let publisher = Publisher {
                fail: fallback_fail,
                items: Arc::default(),
            };
            let logs = LogCapture::default();
            persist_response(&repo, &publisher, &res)
                .with_subscriber(logs.subscriber())
                .await;
            let items = publisher.items.lock().unwrap();
            assert_eq!(items.len(), usize::from(db_fail));
            if db_fail {
                let PubSubItem::CreatingRes(item) = &items[0] else {
                    panic!("expected CreatingRes")
                };
                assert_eq!(item.id, res.id);
                assert_eq!(item.body, res.body);
            }
            let output = logs.contents();
            assert_eq!(
                output.contains("failed to create response in database"),
                db_fail
            );
            assert_eq!(
                output.contains("failed to publish response persistence fallback"),
                fallback_fail
            );
            if db_fail {
                assert!(output.contains(&res.id.to_string()));
            }
            assert!(!output.contains(&res.body));
            assert!(!output.contains(&res.ip_addr));
        }
    }
}
