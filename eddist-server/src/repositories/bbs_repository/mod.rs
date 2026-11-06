mod authed_token;
mod board;
mod response;
mod thread;

pub use authed_token::{AuthedTokenRepository, CreatingAuthedToken};
pub use board::BoardRepository;
pub use eddist_core::domain::pubsub_repository::CreatingThread;
pub use response::ResponseRepository;
pub use thread::{CreateThreadError, ThreadRepository, ThreadStatus};

use sea_orm::DatabaseConnection;

#[async_trait::async_trait]
pub trait BbsRepository:
    BoardRepository + ThreadRepository + ResponseRepository + AuthedTokenRepository
{
}

#[derive(Debug, Clone)]
pub struct BbsRepositoryImpl {
    pub(super) db: DatabaseConnection,
}

impl BbsRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> BbsRepositoryImpl {
        BbsRepositoryImpl { db }
    }
}

impl BbsRepository for BbsRepositoryImpl {}
