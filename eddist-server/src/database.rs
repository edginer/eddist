use std::time::Duration;

use sea_orm::{ConnectOptions, DatabaseConnection, DbErr, SqlxMySqlConnector};
use sqlx::Executor;

/// Create the single MySQL pool shared by repositories and background refresh tasks.
pub async fn connect_database(url: &str) -> Result<DatabaseConnection, DbErr> {
    let mut options = ConnectOptions::new(url);
    options
        .max_connections(8)
        .acquire_timeout(Duration::from_secs(5))
        .sqlx_logging(false)
        .record_stmt_in_spans(false)
        .map_sqlx_mysql_pool_opts(|pool| {
            pool.after_connect(|conn, _| {
                Box::pin(async move {
                    conn.execute("SET SESSION TRANSACTION ISOLATION LEVEL READ COMMITTED")
                        .await?;
                    // The remaining SQLx writes rely on MySQL truncating fractional seconds.
                    conn.execute(
                        "SET SESSION sql_mode = CONCAT(@@sql_mode, ',TIME_TRUNCATE_FRACTIONAL')",
                    )
                    .await?;
                    Ok(())
                })
            })
        });
    SqlxMySqlConnector::connect(options).await
}
