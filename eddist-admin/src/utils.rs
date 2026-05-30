use sqlx::{Database, Transaction};

#[async_trait::async_trait]
pub trait TransactionRepository<T: Database> {
    async fn begin(&self) -> anyhow::Result<Transaction<'_, T>>;
}

/// Implement `TransactionRepository<DB>` for a repository struct that holds a pool as a named
/// field (`$conn`) or index (`0`).
///
/// Usage:
/// ```ignore
/// transaction_repository!(AdminBoardRepositoryImpl, 0, MySql);
/// transaction_repository!(UserRestrictionRepositoryImpl, pool, MySql);
/// ```
#[macro_export]
macro_rules! transaction_repository {
    ($impl_struct:ident, $conn:tt, $database:ident) => {
        #[async_trait::async_trait]
        impl $crate::utils::TransactionRepository<sqlx::$database> for $impl_struct {
            async fn begin(&self) -> anyhow::Result<sqlx::Transaction<'_, sqlx::$database>> {
                let tx = self.$conn.begin().await?;
                Ok(tx)
            }
        }
    };
}
