use sea_orm::DatabaseConnection;

pub(super) async fn create_test_app(
    db: DatabaseConnection,
    redis_conn: redis::aio::ConnectionManager,
) -> anyhow::Result<axum::Router> {
    let s3_creds = aws_sdk_s3::config::Credentials::new("test", "test", None, None, "custom");
    let s3_config = aws_sdk_s3::Config::builder()
        .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
        .credentials_provider(s3_creds)
        .region(aws_sdk_s3::config::Region::new("auto"))
        .endpoint_url("https://test.r2.cloudflarestorage.com")
        .build();

    let app = eddist::app::build_app(
        eddist::config::AppConfig::for_test(),
        eddist::app::AppDependencies {
            db,
            redis_conn,
            s3_client: aws_sdk_s3::Client::from_conf(s3_config),
            s3_bucket_name: "test-bucket".to_string(),
        },
    )
    .await?;
    Ok(app.router)
}
