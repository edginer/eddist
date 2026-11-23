use std::time::Duration;

use eddist::database::connect_database;
use testcontainers::{ImageExt, core::IntoContainerPort, runners::AsyncRunner};
use testcontainers_modules::mysql::Mysql;

#[tokio::test]
async fn production_pool_initializes_each_mysql_session() {
    let mysql = Mysql::default().with_tag("8.0").start().await.unwrap();
    let port = mysql.get_host_port_ipv4(3306.tcp()).await.unwrap();
    let db = connect_database(&format!("mysql://root@127.0.0.1:{port}/test"))
        .await
        .unwrap();
    let pool = db.get_mysql_connection_pool();
    assert_eq!(pool.options().get_max_connections(), 8);
    assert_eq!(pool.options().get_acquire_timeout(), Duration::from_secs(5));

    let mut first = pool.acquire().await.unwrap();
    let mut second = pool.acquire().await.unwrap();
    for connection in [&mut first, &mut second] {
        let (isolation, sql_mode): (String, String) =
            sqlx::query_as("SELECT @@SESSION.transaction_isolation, @@SESSION.sql_mode")
                .fetch_one(&mut **connection)
                .await
                .unwrap();
        assert_eq!(isolation, "READ-COMMITTED");
        assert!(
            sql_mode
                .split(',')
                .any(|mode| mode == "TIME_TRUNCATE_FRACTIONAL")
        );
    }
}
