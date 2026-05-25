mod authed_token;
mod board;
mod response;
mod thread;

pub use authed_token::{AuthedTokenRepository, CreatingAuthedToken};
pub use board::BoardRepository;
pub use eddist_core::domain::pubsub_repository::CreatingThread;
pub use response::ResponseRepository;
pub use thread::{ThreadRepository, ThreadStatus};

use sqlx::MySqlPool;

#[async_trait::async_trait]
pub trait BbsRepository:
    BoardRepository + ThreadRepository + ResponseRepository + AuthedTokenRepository
{
}

#[derive(Debug, Clone)]
pub struct BbsRepositoryImpl {
    pub(super) pool: MySqlPool,
}

impl BbsRepositoryImpl {
    pub fn new(pool: MySqlPool) -> BbsRepositoryImpl {
        BbsRepositoryImpl { pool }
    }
}

impl BbsRepository for BbsRepositoryImpl {}
