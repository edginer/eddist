use chrono::NaiveDate;
use sqlx::MySqlPool;

#[derive(Debug, Clone)]
pub struct DailyStat {
    pub date: NaiveDate,
    pub total_responses: i64,
    pub new_threads: i64,
}

#[async_trait::async_trait]
pub trait StatsRepository: Send + Sync + 'static {
    async fn get_today_stat(&self) -> anyhow::Result<Option<DailyStat>>;
    async fn get_daily_stats(&self, days: u32) -> anyhow::Result<Vec<DailyStat>>;
}

#[derive(Debug, Clone)]
pub struct StatsRepositoryImpl {
    pool: MySqlPool,
}

impl StatsRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl StatsRepository for StatsRepositoryImpl {
    async fn get_today_stat(&self) -> anyhow::Result<Option<DailyStat>> {
        let row = sqlx::query_as!(
            DailyStat,
            r#"SELECT
                date AS "date: NaiveDate",
                total_responses,
                new_threads
            FROM daily_stats
            WHERE date = DATE(CONVERT_TZ(NOW(), '+00:00', '+09:00'))"#,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    async fn get_daily_stats(&self, days: u32) -> anyhow::Result<Vec<DailyStat>> {
        let rows = sqlx::query_as!(
            DailyStat,
            r#"SELECT
                date AS "date: NaiveDate",
                total_responses,
                new_threads
            FROM daily_stats
            WHERE date < DATE(CONVERT_TZ(NOW(), '+00:00', '+09:00'))
            ORDER BY date DESC
            LIMIT ?"#,
            days,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }
}
