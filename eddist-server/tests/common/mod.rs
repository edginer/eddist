use axum_test::TestServer;
use eddist::test_helpers::decode_sjis;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::{MySql, Pool};
use std::time::Duration;
use testcontainers::ImageExt;
use testcontainers::{core::IntoContainerPort, runners::AsyncRunner, ContainerAsync};
use testcontainers_modules::{mysql::Mysql, redis::Redis};

pub struct TestContext {
    pub _mysql_container: ContainerAsync<Mysql>,
    pub _redis_container: ContainerAsync<Redis>,
    pub pool: Pool<MySql>,
    pub server: TestServer,
}

impl TestContext {
    pub async fn new() -> Self {
        std::env::set_var("BASE_URL", "http://localhost:8000");
        std::env::set_var("RUST_ENV", "prod");

        let mysql_container = Mysql::default()
            .with_name("mysql")
            .with_tag("8.0")
            .start()
            .await
            .expect("Failed to start MySQL container");

        println!("Waiting for MySQL to be ready...");

        let mysql_port = mysql_container
            .get_host_port_ipv4(3306.tcp())
            .await
            .expect("Failed to get MySQL port");

        let database_url = format!("mysql://root@127.0.0.1:{mysql_port}/test");

        let pool = MySqlPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(10))
            .connect(&database_url)
            .await
            .expect("Failed to connect to MySQL");

        sqlx::migrate!("../migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        println!("MySQL is ready.");

        let redis_container = Redis::default()
            .with_name("valkey/valkey")
            .with_tag("8.0")
            .start()
            .await
            .expect("Failed to start Redis container");

        println!("Waiting for Redis to be ready...");

        let redis_port = redis_container
            .get_host_port_ipv4(6379.tcp())
            .await
            .expect("Failed to get Redis port");

        let redis_url = format!("redis://127.0.0.1:{}", redis_port);
        let redis_client = redis::Client::open(redis_url).expect("Failed to create Redis client");
        let redis_conn = redis_client
            .get_connection_manager()
            .await
            .expect("Failed to get Redis connection manager");

        println!("Redis is ready.");

        let app = eddist::create_test_app(pool.clone(), redis_conn);
        let mut server = TestServer::new(app).expect("Failed to create test server");
        server.add_header("CF-Connecting-IP", "localhost");
        server.add_header("X-ASN-Num", "1");

        Self {
            _mysql_container: mysql_container,
            _redis_container: redis_container,
            pool,
            server,
        }
    }

    pub async fn get_thread_dat_with_retry(
        &self,
        board_key: &str,
        thread_id: &str,
        assertion_fn: fn(&str) -> bool,
    ) -> anyhow::Result<String> {
        let mut retries = 3;
        let mut last_response = String::new();

        while retries > 0 {
            let response = self
                .server
                .get(&format!("/{board_key}/dat/{thread_id}.dat"))
                .await;

            let resp_bytes = response.as_bytes();
            let resp_text = decode_sjis(resp_bytes);

            println!("Thread DAT content:\n{resp_text}");

            if assertion_fn(&resp_text) {
                return Ok(resp_text);
            } else {
                println!(
                    "Assertion failed for DAT response. Retries left: {}",
                    retries - 1
                );

                last_response = resp_text;
                retries -= 1;
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }

        Err(anyhow::anyhow!(
            "Failed to get correct DAT response after retries. Last response:\n{}",
            last_response
        ))
    }
}
