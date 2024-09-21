use auth_with_code_service::AuthWithCodeService;
use board_info_service::BoardInfoService;
use list_boards_service::ListBoardsService;
use redis::aio::ConnectionManager;

use res_creation_service::ResCreationService;
use thread_creation_service::TheradCreationService;
use thread_list_service::ThreadListService;
use thread_retrieval_service::ThreadRetrievalService;

use crate::{
    error::BbsCgiError,
    repositories::{bbs_pubsub_repository::PubRepository, bbs_repository::BbsRepository},
};

pub(crate) mod auth_with_code_service;
pub(crate) mod board_info_service;
pub(crate) mod list_boards_service;
pub(crate) mod res_creation_service;
pub(crate) mod thread_creation_service;
pub(crate) mod thread_list_service;
pub(crate) mod thread_retrieval_service;

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
pub struct AppServiceContainer<B: BbsRepository + 'static, P: PubRepository> {
    auth_with_code: AuthWithCodeService<B>,
    board_info: BoardInfoService<B>,
    list_boards: ListBoardsService<B>,
    res_creation: ResCreationService<B, P>,
    thread_creation: TheradCreationService<B>,
    thread_list: ThreadListService<B>,
    thread_retrival: ThreadRetrievalService<B>,
}

impl<B: BbsRepository + Clone, P: PubRepository> AppServiceContainer<B, P> {
    pub fn new(bbs_repo: B, redis_conn: ConnectionManager, pub_repo: P) -> Self {
        AppServiceContainer {
            auth_with_code: AuthWithCodeService::new(bbs_repo.clone()),
            board_info: BoardInfoService::new(bbs_repo.clone()),
            list_boards: ListBoardsService::new(bbs_repo.clone()),
            res_creation: ResCreationService::new(bbs_repo.clone(), redis_conn.clone(), pub_repo),
            thread_creation: TheradCreationService::new(bbs_repo.clone(), redis_conn.clone()),
            thread_list: ThreadListService::new(bbs_repo.clone()),
            thread_retrival: ThreadRetrievalService::new(bbs_repo, redis_conn),
        }
    }
}

impl<B: BbsRepository + 'static, P: PubRepository> AppServiceContainer<B, P> {
    pub fn auth_with_code(&self) -> &AuthWithCodeService<B> {
        &self.auth_with_code
    }

    pub fn board_info(&self) -> &BoardInfoService<B> {
        &self.board_info
    }

    pub fn res_creation(&self) -> &ResCreationService<B, P> {
        &self.res_creation
    }

    pub fn thread_creation(&self) -> &TheradCreationService<B> {
        &self.thread_creation
    }

    pub fn thread_list(&self) -> &ThreadListService<B> {
        &self.thread_list
    }

    pub fn thread_retrival(&self) -> &ThreadRetrievalService<B> {
        &self.thread_retrival
    }

    pub fn list_boards(&self) -> &ListBoardsService<B> {
        &self.list_boards
    }
}
