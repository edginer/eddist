use std::sync::Arc;

use eddist_core::{domain::tinker::Tinker, utils::is_thread_pub_enabled};
use metrics::counter;
use uuid::Uuid;

use crate::{
    context::AppContext,
    domain::{
        ng_word::NgWordRestrictable,
        utils::{sanitize_base, sanitize_num_refs},
    },
    error::BbsCgiError,
    repositories::{
        bbs_pubsub_repository::{CreationEventRepository, record_event_publish},
        bbs_repository::{BbsRepository, CreateThreadError, CreatingThread},
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
pub struct ThreadCreationService<T: BbsRepository, E: CreationEventRepository> {
    bbs_repo: T,
    stores: PostingStores,
    event_repo: E,
    ctx: AppContext,
}

impl<T: BbsRepository, E: CreationEventRepository> ThreadCreationService<T, E> {
    pub fn new(bbs_repo: T, stores: PostingStores, event_repo: E, ctx: AppContext) -> Self {
        Self {
            bbs_repo,
            stores,
            event_repo,
            ctx,
        }
    }
}

#[async_trait::async_trait]
impl<T: BbsRepository + Clone, E: CreationEventRepository>
    BbsCgiService<ThreadCreationServiceInput, ThreadCreationServiceOutput>
    for ThreadCreationService<T, E>
{
    async fn execute(
        &self,
        input: ThreadCreationServiceInput,
    ) -> Result<ThreadCreationServiceOutput, BbsCgiError> {
        let bbs_repo = self.bbs_repo.clone();

        let (res_id, th_id) = (Uuid::now_v7(), Uuid::now_v7());

        let preparation = PostingPreparationService::new(
            self.bbs_repo.clone(),
            self.stores.clone(),
            self.event_repo.clone(),
            self.ctx.clone(),
        );
        let PreparedPosting {
            board,
            board_info,
            thread: _,
            created_at,
            authed_token,
            client_info,
            res,
        } = preparation
            .prepare(PostingPreparationInput {
                board_key: &input.board_key,
                target: PostingTarget::Thread {
                    title: &input.title,
                },
                authed_token: input.authed_token.as_deref(),
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

        let unix_time = created_at.timestamp();
        let title = sanitize_thread_name(&input.title);
        let board_key = input.board_key.clone();

        let res_span = board_info.base_response_creation_span_sec as u64;
        let thread_span = board_info.base_thread_creation_span_sec as u64;
        let authed_token_reduced_ip = authed_token.reduced_ip.to_string();
        if self
            .stores
            .rate_limit
            .is_thread_restricted(
                thread_span,
                &authed_token_reduced_ip,
                &input.ip_addr,
                unix_time as u64,
            )
            .await
        {
            return Err(BbsCgiError::TooManyCreatingThreadWithoutTinker);
        }

        let ng_words = self.stores.ng_words.get_ng_words(&input.board_key).await?;
        if (&res, title.clone()).contains_ng_word(&ng_words) {
            return Err(BbsCgiError::NgWordDetected);
        }

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
            moderation_result: None,
        };

        let event_repo = self.event_repo.clone();
        let creating_th_clone = creating_th.clone();

        let db_result = bbs_repo.create_thread(creating_th).await;
        if db_result.is_ok() && is_thread_pub_enabled() {
            let settings = input.settings.clone();
            let moderation = self.ctx.moderation.clone();
            self.ctx
                .background_tasks
                .spawn_detached("thread_event_publish", async move {
                    let moderation_result =
                        if settings.is_enabled(ServerSettingKey::AiModerationOnThread) {
                            let text =
                                format!("{}\n{}", creating_th_clone.title, creating_th_clone.body);
                            moderation.moderate(&settings, &text).await
                        } else {
                            None
                        };
                    let mut creating_th_clone = creating_th_clone;
                    creating_th_clone.moderation_result = moderation_result;
                    record_event_publish(
                        "thread_created",
                        event_repo.publish_thread_created(creating_th_clone).await,
                    );
                });
        }
        db_result.map_err(|e| match e {
            CreateThreadError::ThreadNumberConflict => BbsCgiError::SameTimeThreadCration,
            CreateThreadError::Other(e) => BbsCgiError::Other(e),
        })?;
        let stores = self.stores.clone();
        self.ctx
            .background_tasks
            .spawn("thread_dat_and_limits", async move {
                stores
                    .dat_cache
                    .create(
                        &input.board_key,
                        unix_time as u64,
                        res.get_sjis_bytes(&board.default_name, Some(&title))
                            .get_inner(),
                    )
                    .await?;
                stores
                    .rate_limit
                    .record_res(
                        res_span,
                        &authed_token_reduced_ip,
                        &input.ip_addr,
                        unix_time as u64,
                    )
                    .await?;
                stores
                    .rate_limit
                    .record_thread(
                        thread_span,
                        &authed_token_reduced_ip,
                        &input.ip_addr,
                        unix_time as u64,
                    )
                    .await
            })
            .await
            .map_err(|e| BbsCgiError::Other(e.into()))??;

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

        Ok(ThreadCreationServiceOutput {
            tinker,
            authed_token_id: authed_token.id,
            is_authed_token_bound: authed_token.registered_user_id.is_some(),
        })
    }
}

pub struct ThreadCreationServiceInput {
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
    pub settings: Arc<ServerSettingsSnapshot>,
}

pub struct ThreadCreationServiceOutput {
    pub tinker: Tinker,
    pub authed_token_id: Uuid,
    pub is_authed_token_bound: bool,
}

pub fn sanitize_thread_name(name: &str) -> String {
    sanitize_base(&sanitize_num_refs(name), false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_thread_name() {
        assert_eq!(sanitize_thread_name("normal title"), "normal title");
        assert_eq!(
            sanitize_thread_name("<script>alert()</script>"),
            "&lt;script&gt;alert()&lt;/script&gt;"
        );
        assert_eq!(
            sanitize_thread_name("title with\nnewline"),
            "title withnewline"
        );
        assert_eq!(sanitize_thread_name("title&#10;test"), "titletest");
        // Ensure <br> is escaped (becomes &lt;br&gt;) and never appears raw in title
        assert_eq!(sanitize_thread_name("some<br>text"), "some&lt;br&gt;text");
        // Ensure title never contains raw <br>
        assert!(!sanitize_thread_name("title\ntest").contains("<br>"));
        assert!(!sanitize_thread_name("title<br>test").contains("<br>"));

        // XSS prevention: numeric character references for < and > must be escaped
        // &#60; = '<', &#62; = '>', &#x3C; = '<', &#x3E; = '>'
        assert_eq!(
            sanitize_thread_name("&#60;script&#62;alert()&#60;/script&#62;"),
            "&lt;script&gt;alert()&lt;/script&gt;"
        );
        assert_eq!(
            sanitize_thread_name("&#x3C;script&#x3E;alert()&#x3C;/script&#x3E;"),
            "&lt;script&gt;alert()&lt;/script&gt;"
        );
        // Mixed: some literal, some numeric refs
        assert_eq!(
            sanitize_thread_name("<img src=x onerror=&#34;alert()&#34;>"),
            "&lt;img src=x onerror=&quot;alert()&quot;&gt;"
        );
    }
}
