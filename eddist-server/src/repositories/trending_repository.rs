use sqlx::MySqlPool;

#[derive(Debug, Clone)]
pub struct TrendingThread {
    pub thread_id: String,
    pub board_id: String,
    pub board_key: String,
    pub board_name: String,
    pub thread_number: i64,
    pub title: String,
    pub response_count: i32,
    pub recent_response_count: i64,
}

#[async_trait::async_trait]
pub trait TrendingRepository: Send + Sync + 'static {
    async fn get_trending_threads(
        &self,
        window_hours: u32,
        fetch_limit: u32,
    ) -> anyhow::Result<Vec<TrendingThread>>;
}

#[derive(Debug, Clone)]
pub struct TrendingRepositoryImpl {
    pool: MySqlPool,
}

impl TrendingRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl TrendingRepository for TrendingRepositoryImpl {
    async fn get_trending_threads(
        &self,
        window_hours: u32,
        fetch_limit: u32,
    ) -> anyhow::Result<Vec<TrendingThread>> {
        let rows = sqlx::query_as!(
            TrendingThread,
            r#"SELECT
                BIN_TO_UUID(t.id)       AS "thread_id!: String",
                BIN_TO_UUID(t.board_id) AS "board_id!: String",
                b.board_key             AS board_key,
                b.name                  AS board_name,
                t.thread_number         AS thread_number,
                t.title                 AS title,
                t.response_count        AS response_count,
                COUNT(r.id)             AS "recent_response_count: i64"
            FROM threads t
            INNER JOIN boards b ON b.id = t.board_id
            INNER JOIN responses r
                   ON r.thread_id = t.id
                   AND r.created_at >= DATE_SUB(NOW(), INTERVAL ? HOUR)
                   AND r.is_abone = FALSE
            WHERE t.active   = TRUE
              AND t.archived = FALSE
            GROUP BY t.id, t.board_id, b.board_key, b.name, t.thread_number, t.title, t.response_count
            ORDER BY COUNT(r.id) DESC
            LIMIT ?"#,
            window_hours,
            fetch_limit,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }
}
