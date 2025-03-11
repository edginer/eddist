use auth_with_code_service::AuthWithCodeService;
use board_info_service::BoardInfoService;
use kako_thread_retrieval_service::KakoThreadRetrievalService;
use list_boards_service::ListBoardsService;
use metadent_thread_list_service::MetadentThreadListService;
use redis::aio::ConnectionManager;

use res_creation_service::ResCreationService;
use s3::Bucket;
use thread_creation_service::TheradCreationService;
use thread_list_service::ThreadListService;
use thread_retrieval_service::ThreadRetrievalService;
use user_reg_authz_idp_callback_service::UserRegAuthzIdpCallbackService;
use user_reg_idp_redirection_service::UserRegIdpRedirectionService;
use user_reg_temp_url_service::UserRegTempUrlService;

use crate::{
    error::BbsCgiError,
    repositories::{
        bbs_pubsub_repository::PubRepository, bbs_repository::BbsRepository,
        idp_repository::IdpRepository, user_repository::UserRepository,
    },
};

pub(crate) mod auth_with_code_service;
pub(crate) mod board_info_service;
pub(crate) mod kako_thread_retrieval_service;
pub(crate) mod list_boards_service;
pub(crate) mod metadent_thread_list_service;
pub(crate) mod res_creation_service;
pub(crate) mod thread_creation_service;
pub(crate) mod thread_list_service;
pub(crate) mod thread_retrieval_service;
pub(crate) mod user_reg_authz_idp_callback_service;
pub(crate) mod user_reg_idp_redirection_service;
pub(crate) mod user_reg_temp_url_service;

#[mockall::automock]
#[async_trait::async_trait]
pub trait AppService<I: Send + Sync, O: Send + Sync> {
    async fn execute(&self, input: I) -> anyhow::Result<O>;
}

#[mockall::automock]
#[async_trait::async_trait]
pub trait BbsCgiService<I: Send + Sync, O: Send + Sync> {
    async fn execute(&self, input: I) -> Result<O, BbsCgiError>;
}

#[derive(Clone)]
pub struct AppServiceContainer<
    B: BbsRepository + 'static,
    U: UserRepository + 'static,
    I: IdpRepository + 'static,
    P: PubRepository,
> {
    auth_with_code: AuthWithCodeService<B>,
    board_info: BoardInfoService<B>,
    list_boards: ListBoardsService<B>,
    res_creation: ResCreationService<B, U, P>,
    thread_creation: TheradCreationService<B, U>,
    thread_list: ThreadListService<B>,
    metadent_thread_list: MetadentThreadListService<B>,
    thread_retrival: ThreadRetrievalService<B>,
    kako_thread_retrieval: KakoThreadRetrievalService,

    user_reg_temp_url: UserRegTempUrlService<I>,
    user_reg_idp_redirection: UserRegIdpRedirectionService<I>,
    user_reg_authz_idp_callback: UserRegAuthzIdpCallbackService<I, U>,
}

impl<
        B: BbsRepository + Clone,
        U: UserRepository + Clone,
        I: IdpRepository + Clone,
        P: PubRepository,
    > AppServiceContainer<B, U, I, P>
{
    pub fn new(
        bbs_repo: B,
        user_repo: U,
        idp_repo: I,
        redis_conn: ConnectionManager,
        pub_repo: P,
        bucket: Bucket,
    ) -> Self {
        AppServiceContainer {
            auth_with_code: AuthWithCodeService::new(bbs_repo.clone()),
            board_info: BoardInfoService::new(bbs_repo.clone()),
            list_boards: ListBoardsService::new(bbs_repo.clone()),
            res_creation: ResCreationService::new(
                bbs_repo.clone(),
                user_repo.clone(),
                redis_conn.clone(),
                pub_repo,
            ),
            thread_creation: TheradCreationService::new(
                bbs_repo.clone(),
                user_repo.clone(),
                redis_conn.clone(),
            ),
            thread_list: ThreadListService::new(bbs_repo.clone()),
            metadent_thread_list: MetadentThreadListService::new(bbs_repo.clone()),
            thread_retrival: ThreadRetrievalService::new(bbs_repo, redis_conn.clone()),
            kako_thread_retrieval: KakoThreadRetrievalService::new(bucket),

            user_reg_temp_url: UserRegTempUrlService::new(idp_repo.clone(), redis_conn.clone()),
            user_reg_idp_redirection: UserRegIdpRedirectionService::new(
                idp_repo.clone(),
                redis_conn.clone(),
            ),
            user_reg_authz_idp_callback: UserRegAuthzIdpCallbackService::new(
                idp_repo, user_repo, redis_conn,
            ),
        }
    }
}

impl<
        B: BbsRepository + 'static,
        U: UserRepository + 'static,
        I: IdpRepository + 'static,
        P: PubRepository,
    > AppServiceContainer<B, U, I, P>
{
    pub fn auth_with_code(&self) -> &AuthWithCodeService<B> {
        &self.auth_with_code
    }

    pub fn board_info(&self) -> &BoardInfoService<B> {
        &self.board_info
    }

    pub fn res_creation(&self) -> &ResCreationService<B, U, P> {
        &self.res_creation
    }

    pub fn thread_creation(&self) -> &TheradCreationService<B, U> {
        &self.thread_creation
    }

    pub fn thread_list(&self) -> &ThreadListService<B> {
        &self.thread_list
    }

    pub fn metadent_thread_list(&self) -> &MetadentThreadListService<B> {
        &self.metadent_thread_list
    }

    pub fn thread_retrival(&self) -> &ThreadRetrievalService<B> {
        &self.thread_retrival
    }

    pub fn list_boards(&self) -> &ListBoardsService<B> {
        &self.list_boards
    }

    pub fn kako_thread_retrieval(&self) -> &KakoThreadRetrievalService {
        &self.kako_thread_retrieval
    }

    pub fn user_reg_temp_url(&self) -> &UserRegTempUrlService<I> {
        &self.user_reg_temp_url
    }

    pub fn user_reg_idp_redirection(&self) -> &UserRegIdpRedirectionService<I> {
        &self.user_reg_idp_redirection
    }

    pub fn user_reg_authz_idp_callback(&self) -> &UserRegAuthzIdpCallbackService<I, U> {
        &self.user_reg_authz_idp_callback
    }
}
