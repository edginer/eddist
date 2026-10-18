use std::borrow::Cow;

use chrono::{DateTime, Utc};
use eddist_core::domain::{
    board::{Board, BoardInfo},
    client_info::ClientInfo,
    tinker::Tinker,
};

use crate::{
    context::AppContext,
    domain::{
        authed_token::AuthedToken,
        posting_policy::{self, PostingKind},
        res::{AuthorIdInitialized, Res},
        res_core::ResCore,
        service::{
            bbscgi_auth_service::BbsCgiAuthService, board_info_service::BoardInfoService,
            email_auth_restriction_service::EmailAuthRestrictionService,
        },
        thread::Thread,
    },
    error::{BbsCgiError, NotFoundParamType},
    repositories::{bbs_pubsub_repository::CreationEventRepository, bbs_repository::BbsRepository},
    stores::PostingStores,
};

use super::{
    server_settings_cache::ServerSettingsSnapshot,
    validation::{check_userreg, resolve_cap_name},
};

/// The operation-specific target lookup performed after the board checks.
pub enum PostingTarget<'a> {
    /// A reply: the thread must exist and be active.
    Res { thread_number: u64 },
    /// A new thread carrying the raw title, validated with the body.
    Thread { title: &'a str },
}

impl PostingTarget<'_> {
    fn kind(&self) -> PostingKind {
        match self {
            PostingTarget::Res { .. } => PostingKind::Res,
            PostingTarget::Thread { .. } => PostingKind::Thread,
        }
    }

    fn title(&self) -> Option<&str> {
        match self {
            PostingTarget::Res { .. } => None,
            PostingTarget::Thread { title } => Some(title),
        }
    }
}

pub struct PostingPreparationInput<'a> {
    pub board_key: &'a str,
    pub target: PostingTarget<'a>,
    pub authed_token: Option<&'a str>,
    pub name: &'a str,
    pub mail: &'a str,
    pub body: &'a str,
    pub tinker: Option<&'a Tinker>,
    pub ip_addr: &'a str,
    pub user_agent: &'a str,
    pub asn_num: u32,
    pub settings: &'a ServerSettingsSnapshot,
}

/// Everything the shared preparation checked and resolved before the posting is stored.
pub struct PreparedPosting {
    pub board: Board,
    pub board_info: BoardInfo,
    /// The target thread for a reply; `None` for a new thread.
    pub thread: Option<Thread>,
    pub created_at: DateTime<Utc>,
    pub authed_token: AuthedToken,
    pub client_info: ClientInfo,
    /// The posting with its author id and cap resolved.
    pub res: Res<AuthorIdInitialized>,
}

#[derive(Clone)]
pub struct PostingPreparationService<T: BbsRepository, E: CreationEventRepository> {
    bbs_repo: T,
    stores: PostingStores,
    event_repo: E,
    ctx: AppContext,
}

impl<T: BbsRepository + Clone, E: CreationEventRepository> PostingPreparationService<T, E> {
    pub fn new(bbs_repo: T, stores: PostingStores, event_repo: E, ctx: AppContext) -> Self {
        Self {
            bbs_repo,
            stores,
            event_repo,
            ctx,
        }
    }

    pub async fn prepare(
        &self,
        input: PostingPreparationInput<'_>,
    ) -> Result<PreparedPosting, BbsCgiError> {
        let board_info_svc =
            BoardInfoService::new(self.bbs_repo.clone(), self.ctx.board_info.clone());
        let (board, board_info) = board_info_svc
            .get_board_info_by_key(input.board_key)
            .await?
            .ok_or_else(|| BbsCgiError::from(NotFoundParamType::Board))?;

        if board_info.read_only {
            return Err(BbsCgiError::ReadOnlyBoard);
        }

        let created_at = Utc::now();

        // A reply fetches its target thread here; a new thread checks that no other
        // thread shares its creation timestamp.
        let thread = match &input.target {
            PostingTarget::Res { thread_number } => {
                let thread = self
                    .bbs_repo
                    .get_thread_by_board_key_and_thread_number(input.board_key, *thread_number)
                    .await
                    .map_err(BbsCgiError::Other)?
                    .ok_or_else(|| BbsCgiError::from(NotFoundParamType::Thread))?;
                if !thread.active {
                    return Err(BbsCgiError::InactiveThread);
                }
                Some(thread)
            }
            PostingTarget::Thread { .. } => {
                if self
                    .bbs_repo
                    .get_thread_by_board_key_and_thread_number(
                        input.board_key,
                        created_at.timestamp() as u64,
                    )
                    .await
                    .map_err(BbsCgiError::Other)?
                    .is_some()
                {
                    return Err(BbsCgiError::SameTimeThreadCration);
                }
                None
            }
        };

        let res_core = ResCore {
            from: input.name,
            mail: input.mail,
            body: Cow::Borrowed(input.body),
        };
        let client_info = ClientInfo {
            user_agent: input.user_agent.to_string(),
            asn_num: input.asn_num,
            ip_addr: input.ip_addr.to_string(),
            tinker: input.tinker.map(|t| Box::new(t.clone())),
        };

        posting_policy::validate_content_length(&res_core, input.target.title(), &board_info)?;
        posting_policy::validate_client_info(
            &client_info,
            &board_info,
            input.target.kind(),
            self.ctx.config.restrict_thread_creation_on_no_tinker,
        )?;

        let force_metadent = board_info
            .force_metadent_type
            .as_deref()
            .and_then(|s| s.parse().ok());
        let res = match &thread {
            Some(th) => Res::new_from_res(
                res_core,
                input.board_key,
                created_at,
                force_metadent.unwrap_or_else(|| (&th.metadent as &str).into()),
                client_info.clone(),
                input.authed_token.map(str::to_string),
                false,
            ),
            None => Res::new_from_thread(
                res_core,
                input.board_key,
                created_at,
                client_info.clone(),
                input.authed_token.map(str::to_string),
                false,
                force_metadent,
            ),
        };

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
                input.ip_addr.to_string(),
                input.user_agent.to_string(),
                input.asn_num as i32,
                created_at,
                input.settings,
            )
            .await?;

        let email_auth_service = EmailAuthRestrictionService::new(self.stores.auth_state.clone());
        email_auth_service
            .check_and_enforce_restriction(
                res.is_email_authed(),
                input.user_agent,
                &authed_token.token,
                input.ip_addr,
            )
            .await?;

        if let Some(err) = check_userreg(
            input.body,
            &authed_token,
            self.stores.auth_state.as_ref(),
            input.settings,
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
            input.board_key,
            &self.ctx.config.tinker_secret,
        )
        .await?;
        let res = res.set_author_id(&authed_token, cap_name);

        Ok(PreparedPosting {
            board,
            board_info,
            thread,
            created_at,
            authed_token,
            client_info,
            res,
        })
    }
}
