use std::time::Duration;

use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};

/// Create the single pool shared by repositories and background refresh tasks.
pub async fn connect_database(url: &str) -> Result<DatabaseConnection, DbErr> {
    let mut options = ConnectOptions::new(url);
    options
        .max_connections(8)
        .acquire_timeout(Duration::from_secs(5))
        // Posting/authentication payloads must not be recorded as SQL bind values.
        .sqlx_logging(false)
        .record_stmt_in_spans(false);
    Database::connect(options).await
}
