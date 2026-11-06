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
use sqlx::MySqlPool;

#[async_trait::async_trait]
pub trait BbsRepository:
    BoardRepository + ThreadRepository + ResponseRepository + AuthedTokenRepository
{
}

#[derive(Debug, Clone)]
pub struct BbsRepositoryImpl {
    pub(super) pool: MySqlPool,
    pub(super) db: DatabaseConnection,
}

impl BbsRepositoryImpl {
    pub fn new(pool: MySqlPool, db: DatabaseConnection) -> BbsRepositoryImpl {
        BbsRepositoryImpl { pool, db }
    }
}

impl BbsRepository for BbsRepositoryImpl {}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{
        ConnectionTrait, SqlxMySqlConnector,
        sea_query::{Expr, Query},
    };

    #[tokio::test]
    async fn closing_sqlx_pool_also_closes_the_orm_connection() {
        // A lazy pool keeps this ownership check independent of a running DB.
        let pool = sqlx::mysql::MySqlPoolOptions::new()
            .max_connections(8)
            .connect_lazy("mysql://localhost/test")
            .unwrap();
        let db = SqlxMySqlConnector::from_sqlx_mysql_pool(pool.clone());
        let repo = BbsRepositoryImpl::new(pool.clone(), db);
        pool.close().await;
        assert!(repo.pool.is_closed());
        assert!(repo.db.get_mysql_connection_pool().is_closed());
        assert!(
            repo.db
                .query_one(&Query::select().expr(Expr::val(1)).to_owned())
                .await
                .is_err()
        );
    }
}
