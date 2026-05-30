pub mod archive_service;
pub mod authed_token_service;
pub mod board_service;
pub mod content_admin_service;
pub mod moderation_service;
pub mod thread_service;
pub mod user_service;

use std::sync::Arc;

use crate::{AdminRepos, ContentRepos, ModerationRepos};

use self::{
    archive_service::{AdminArchiveService, ArchiveServiceImpl},
    authed_token_service::{AuthedTokenService, AuthedTokenServiceImpl},
    board_service::{BoardService, BoardServiceImpl},
    content_admin_service::{ContentAdminService, ContentAdminServiceImpl},
    moderation_service::{ModerationService, ModerationServiceImpl},
    thread_service::{ThreadService, ThreadServiceImpl},
    user_service::{UserService, UserServiceImpl},
};

/// Container for all domain services. Add to `AppState` to give handlers access.
#[derive(Clone)]
pub struct AppServiceContainer {
    pub board: Arc<dyn BoardService>,
    pub thread: Arc<dyn ThreadService>,
    pub archive: Arc<dyn AdminArchiveService>,
    pub moderation: Arc<dyn ModerationService>,
    pub authed_token: Arc<dyn AuthedTokenService>,
    pub user: Arc<dyn UserService>,
    pub content_admin: Arc<dyn ContentAdminService>,
}

impl AppServiceContainer {
    pub fn new(
        content: ContentRepos,
        moderation: ModerationRepos,
        admin: AdminRepos,
        redis_conn: redis::aio::ConnectionManager,
    ) -> Self {
        Self {
            board: Arc::new(BoardServiceImpl::new(content.board.clone())),
            thread: Arc::new(ThreadServiceImpl::new(
                content.thread.clone(),
                content.response.clone(),
                redis_conn.clone(),
            )),
            archive: Arc::new(ArchiveServiceImpl::new(
                content.thread.clone(),
                content.response.clone(),
                content.archive.clone(),
            )),
            moderation: Arc::new(ModerationServiceImpl::new(
                moderation.ng_word.clone(),
                moderation.cap.clone(),
                moderation.user_restriction.clone(),
            )),
            authed_token: Arc::new(AuthedTokenServiceImpl::new(
                moderation.authed_token.clone(),
                redis_conn,
            )),
            user: Arc::new(UserServiceImpl::new(admin.user.clone())),
            content_admin: Arc::new(ContentAdminServiceImpl::new(
                admin.notice.clone(),
                admin.terms.clone(),
                admin.server_settings.clone(),
                admin.idp.clone(),
                admin.captcha_config.clone(),
            )),
        }
    }
}
