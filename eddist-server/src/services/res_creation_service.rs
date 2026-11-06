use std::sync::Arc;

use eddist_core::{
    domain::{
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
    domain::{ng_word::NgWordRestrictable, posting_policy},
    error::{BbsCgiError, NotFoundParamType},
    repositories::{
        bbs_pubsub_repository::{CreationEventRepository, PubRepository, record_event_publish},
        bbs_repository::{BbsRepository, ResponseRepository},
    },
    stores::PostingStores,
};

use super::{
    BbsCgiService,
    posting_preparation_service::{
        PostingPreparationInput, PostingPreparationService, PostingTarget, PreparedPosting,
    },
    server_settings_cache::{ServerSettingKey, ServerSettingsSnapshot},
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

        let preparation = PostingPreparationService::new(
            self.bbs_repo.clone(),
            self.stores.clone(),
            self.event_repo.clone(),
            self.ctx.clone(),
        );
        let PreparedPosting {
            board,
            board_info,
            thread,
            created_at,
            authed_token,
            client_info,
            mut res,
        } = preparation
            .prepare(PostingPreparationInput {
                board_key: &input.board_key,
                target: PostingTarget::Res {
                    thread_number: input.thread_number,
                },
                authed_token: input.authed_token_cookie.as_deref(),
                name: &input.name,
                mail: &input.mail,
                body: &input.body,
                tinker: input.tinker.as_ref(),
                ip_addr: &input.ip_addr,
                user_agent: &input.user_agent,
                asn_num: input.asn_num,
                settings: &input.settings,
            })
            .await?;
        // The reply target lookup in the preparation always yields a thread.
        let th = thread.ok_or_else(|| BbsCgiError::from(NotFoundParamType::Thread))?;

        posting_policy::validate_res_images(&mut res, input.tinker.as_ref())?;
        posting_policy::validate_res_anchors(&input.body, input.tinker.as_ref())?;

        let ng_words = self.stores.ng_words.get_ng_words(&input.board_key).await?;
        if res.contains_ng_word(&ng_words) {
            return Err(BbsCgiError::NgWordDetected);
        }

        // Level 1 users: 30 seconds, level 2+ users: base_response_creation_span_sec
        let response_span_sec =
            posting_policy::res_creation_span_sec(input.tinker.as_ref(), &board_info);

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
        let moderation = self.ctx.moderation.clone();

        self.ctx
            .background_tasks
            .spawn_detached("res_persistence", async move {
                persist_response(&bbs_repo, &pub_repo, &cres).await;

                if is_res_pub_enabled() {
                    let moderation_result =
                        if settings.is_enabled(ServerSettingKey::AiModerationOnRes) {
                            moderation.moderate(&settings, &cres.body).await
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
    use chrono::Utc;
    use eddist_core::domain::{client_info::ClientInfo, res::ResView};
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
