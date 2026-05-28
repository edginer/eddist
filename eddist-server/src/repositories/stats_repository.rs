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
    async fn flush_board_stats(&self, snapshot: &[(String, i64, i64)]) -> anyhow::Result<()>;
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

    async fn flush_board_stats(&self, snapshot: &[(String, i64, i64)]) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;

        for (board_key, response_delta, thread_delta) in snapshot {
            sqlx::query!(
                "INSERT INTO daily_stats (date, board_key, total_responses, new_threads) \
                 VALUES (DATE(CONVERT_TZ(NOW(), '+00:00', '+09:00')), ?, ?, ?) \
                 ON DUPLICATE KEY UPDATE \
                 total_responses = total_responses + VALUES(total_responses), \
                 new_threads = new_threads + VALUES(new_threads)",
                board_key,
                response_delta,
                thread_delta,
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }
}
