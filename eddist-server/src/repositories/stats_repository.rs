use chrono::NaiveDate;
use sqlx::MySqlPool;

#[derive(Debug, Clone)]
pub struct BoardDailyStat {
    pub board_key: String,
    pub date: NaiveDate,
    pub total_responses: i64,
    pub new_threads: i64,
}

#[async_trait::async_trait]
pub trait StatsRepository: Send + Sync + 'static {
    async fn get_today_stats_per_board(&self) -> anyhow::Result<Vec<BoardDailyStat>>;
    async fn get_daily_stats_per_board(&self, days: u32) -> anyhow::Result<Vec<BoardDailyStat>>;
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
    async fn get_today_stats_per_board(&self) -> anyhow::Result<Vec<BoardDailyStat>> {
        let rows = sqlx::query_as!(
            BoardDailyStat,
            r#"SELECT
                board_key,
                date AS "date: NaiveDate",
                total_responses,
                new_threads
            FROM daily_stats
            WHERE date = DATE(CONVERT_TZ(NOW(), '+00:00', '+09:00'))"#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    async fn get_daily_stats_per_board(&self, days: u32) -> anyhow::Result<Vec<BoardDailyStat>> {
        let rows = sqlx::query_as!(
            BoardDailyStat,
            r#"SELECT
                board_key,
                date AS "date: NaiveDate",
                total_responses,
                new_threads
            FROM daily_stats
            WHERE date >= DATE_SUB(DATE(CONVERT_TZ(NOW(), '+00:00', '+09:00')), INTERVAL ? DAY)
              AND date < DATE(CONVERT_TZ(NOW(), '+00:00', '+09:00'))
            ORDER BY date DESC, board_key"#,
            days,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }
}
