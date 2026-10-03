use chrono::NaiveDate;
#[cfg(not(feature = "backend-postgres"))]
use sqlx::MySqlPool;
#[cfg(feature = "backend-postgres")]
use sqlx::PgPool;

#[cfg_attr(feature = "backend-postgres", derive(sqlx::FromRow))]
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

#[cfg(not(feature = "backend-postgres"))]
#[derive(Debug, Clone)]
pub struct StatsRepositoryImpl {
    pool: MySqlPool,
}

#[cfg(not(feature = "backend-postgres"))]
impl StatsRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[cfg(not(feature = "backend-postgres"))]
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

#[cfg(feature = "backend-postgres")]
#[derive(Debug, Clone)]
pub struct StatsRepositoryPgImpl {
    pool: PgPool,
}

#[cfg(feature = "backend-postgres")]
impl StatsRepositoryPgImpl {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[cfg(feature = "backend-postgres")]
#[async_trait::async_trait]
impl StatsRepository for StatsRepositoryPgImpl {
    async fn get_today_stats_per_board(&self) -> anyhow::Result<Vec<BoardDailyStat>> {
        let rows = sqlx::query_as!(
            BoardDailyStat,
            r#"
            SELECT board_key, date AS "date: NaiveDate", total_responses, new_threads
            FROM daily_stats
            WHERE date = (CURRENT_TIMESTAMP AT TIME ZONE 'Asia/Tokyo')::date
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    async fn get_daily_stats_per_board(&self, days: u32) -> anyhow::Result<Vec<BoardDailyStat>> {
        let rows = sqlx::query_as!(
            BoardDailyStat,
            r#"
            SELECT board_key, date AS "date: NaiveDate", total_responses, new_threads
            FROM daily_stats
            WHERE date >= ((CURRENT_TIMESTAMP AT TIME ZONE 'Asia/Tokyo')::date - $1::int)
              AND date < (CURRENT_TIMESTAMP AT TIME ZONE 'Asia/Tokyo')::date
            ORDER BY date DESC, board_key
            "#,
            days as i32,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    async fn flush_board_stats(&self, snapshot: &[(String, i64, i64)]) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;

        for (board_key, response_delta, thread_delta) in snapshot {
            sqlx::query!(
                r#"
                INSERT INTO daily_stats (date, board_key, total_responses, new_threads)
                VALUES ((CURRENT_TIMESTAMP AT TIME ZONE 'Asia/Tokyo')::date, $1, $2, $3)
                ON CONFLICT (date, board_key) DO UPDATE SET
                    total_responses = daily_stats.total_responses + EXCLUDED.total_responses,
                    new_threads = daily_stats.new_threads + EXCLUDED.new_threads
                "#,
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
