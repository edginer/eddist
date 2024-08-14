use sqlx::MySqlPool;

pub(crate) struct Repository(MySqlPool);

impl Repository {
    pub fn new(pool: MySqlPool) -> Self {
        Self(pool)
    }
}

impl Repository {
    pub async fn get_all_boards_info(&self) -> anyhow::Result<Vec<SelectionBoardInfo>> {
        let boards = sqlx::query_as!(
            SelectionBoardInfo,
            r#"
            SELECT
                b.board_key AS board_key,
                bi.threads_archive_cron AS threads_archive_cron,
                bi.threads_archive_trigger_thread_count AS threads_archive_trigger_thread_count
            FROM
                boards AS b
            JOIN
                boards_info AS bi
            ON
                b.id = bi.id
            "#,
        )
        .fetch_all(&self.0)
        .await?;
        Ok(boards)
    }

    pub async fn update_threads_to_inactive(
        &self,
        board_key: &str,
        max_thread_count: u32,
    ) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
            UPDATE threads SET archived = 1 WHERE active = 0
            "#,
        )
        .execute(&self.0)
        .await?;

        sqlx::query!(
            r#"
            UPDATE threads SET archived = 1, active = 0 WHERE id IN (
                SELECT id FROM (
                    SELECT id
                    FROM threads
                    WHERE board_id = (SELECT id FROM boards WHERE board_key = ?)
                    AND archived = 0
                    ORDER BY last_modified_at DESC
                    LIMIT ?
                ) AS tmp
            )
            "#,
            board_key,
            max_thread_count,
        )
        .execute(&self.0)
        .await?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SelectionBoardInfo {
    pub board_key: String,
    pub threads_archive_cron: Option<String>,
    pub threads_archive_trigger_thread_count: Option<i32>,
}
