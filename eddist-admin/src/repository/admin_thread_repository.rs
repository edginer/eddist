use chrono::Utc;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::models::Thread;

use super::admin_bbs_repository::SelectionThread;

#[async_trait::async_trait]
pub trait AdminThreadRepository: Send + Sync {
    async fn get_threads_by_thread_id(
        &self,
        board_key: &str,
        thread_numbers: Option<Vec<u64>>,
    ) -> anyhow::Result<Vec<Thread>>;
    async fn get_archived_threads_by_thread_id(
        &self,
        board_key: &str,
        thread_numbers: Option<Vec<u64>>,
    ) -> anyhow::Result<Vec<Thread>>;
    async fn get_archived_threads_by_filter(
        &self,
        board_key: &str,
        keyword: Option<&str>,
        range: (Option<chrono::DateTime<Utc>>, Option<chrono::DateTime<Utc>>),
        page: u64,
        limit: u64,
    ) -> anyhow::Result<Vec<Thread>>;
    async fn compact_threads(&self, board_key: &str, target_count: u32) -> anyhow::Result<()>;
}

#[derive(Clone)]
pub struct AdminThreadRepositoryImpl(pub(crate) MySqlPool);

impl AdminThreadRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self(pool)
    }
}

fn selection_thread_to_thread(thread: SelectionThread) -> Thread {
    Thread {
        id: Uuid::from_slice(&thread.id).unwrap(),
        board_id: Uuid::from_slice(&thread.board_id).unwrap(),
        thread_number: thread.thread_number as u64,
        last_modified: thread.last_modified_at,
        sage_last_modified: thread.sage_last_modified_at,
        title: thread.title,
        authed_token_id: Uuid::from_slice(&thread.authed_token_id).unwrap(),
        metadent: thread.metadent,
        response_count: thread.response_count as u32,
        no_pool: thread.no_pool,
        archived: thread.archived,
        active: thread.active,
    }
}

#[async_trait::async_trait]
impl AdminThreadRepository for AdminThreadRepositoryImpl {
    async fn get_threads_by_thread_id(
        &self,
        board_key: &str,
        thread_numbers: Option<Vec<u64>>,
    ) -> anyhow::Result<Vec<Thread>> {
        let pool = &self.0;

        let thread_numbers_where = if let Some(thread_numbers) = &thread_numbers {
            let mut initial = "AND thread_number IN (".to_string();
            initial.push_str(
                &thread_numbers
                    .iter()
                    .map(|_| "?")
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            initial.push(')');
            initial
        } else {
            "".to_string()
        };

        let query = format!(
            r#"
            SELECT
                *
            FROM
                threads
            WHERE
                board_id = (
                    SELECT
                        id
                    FROM
                        boards
                    WHERE
                        board_key = ?
                )
            {thread_numbers_where}
            "#
        );

        let mut query = sqlx::query_as::<_, SelectionThread>(&query);

        query = query.bind(board_key);
        if let Some(thread_numbers) = &thread_numbers {
            for thread_number in thread_numbers {
                query = query.bind(thread_number);
            }
        }

        let selected_threads = query.fetch_all(pool).await?;
        Ok(selected_threads
            .into_iter()
            .map(selection_thread_to_thread)
            .collect())
    }

    async fn get_archived_threads_by_thread_id(
        &self,
        board_key: &str,
        thread_numbers: Option<Vec<u64>>,
    ) -> anyhow::Result<Vec<Thread>> {
        let pool = &self.0;

        let thread_numbers_where = if let Some(thread_numbers) = &thread_numbers {
            let mut initial = "AND thread_number IN (".to_string();
            initial.push_str(
                &thread_numbers
                    .iter()
                    .map(|_| "?")
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            initial.push(')');
            initial
        } else {
            "".to_string()
        };

        let query = format!(
            r#"
            SELECT
                *
            FROM
                archived_threads
            WHERE
                board_id = (
                    SELECT
                        id
                    FROM
                        boards
                    WHERE
                        board_key = ?
                )
            {thread_numbers_where}
            "#
        );

        let mut query = sqlx::query_as::<_, SelectionThread>(&query);

        query = query.bind(board_key);
        if let Some(thread_numbers) = &thread_numbers {
            for thread_number in thread_numbers {
                query = query.bind(thread_number);
            }
        }

        let selected_threads = query.fetch_all(pool).await?;
        Ok(selected_threads
            .into_iter()
            .map(selection_thread_to_thread)
            .collect())
    }

    async fn get_archived_threads_by_filter(
        &self,
        board_key: &str,
        keyword: Option<&str>,
        range: (Option<chrono::DateTime<Utc>>, Option<chrono::DateTime<Utc>>),
        page: u64,
        limit: u64,
    ) -> anyhow::Result<Vec<Thread>> {
        let pool = &self.0;

        let mut query = r#"
            SELECT
                *
            FROM
                archived_threads
            WHERE
                board_id = (
                    SELECT
                        id
                    FROM
                        boards
                    WHERE
                        board_key = ?
                )
            "#
        .to_string();

        if keyword.is_some() {
            query.push_str("AND title LIKE ? ");
        }

        if matches!(range, (Some(_), Some(_))) {
            query.push_str("AND last_modified_at BETWEEN ? AND ? ");
        }

        query.push_str("ORDER BY last_modified_at DESC ");
        query.push_str("LIMIT ? OFFSET ?");

        let mut query = sqlx::query_as::<_, SelectionThread>(&query);

        query = query.bind(board_key);
        if let Some(keyword) = keyword {
            query = query.bind(format!("%{}%", keyword));
        }
        if let (Some(start), Some(end)) = range {
            query = query.bind(start).bind(end);
        }
        query = query.bind(limit).bind(page * limit);

        let selected_threads = query.fetch_all(pool).await?;
        Ok(selected_threads
            .into_iter()
            .map(selection_thread_to_thread)
            .collect())
    }

    async fn compact_threads(&self, board_key: &str, target_count: u32) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
            UPDATE threads SET archived = 1, active = 0 WHERE id IN (
                SELECT id FROM (
                    SELECT id
                    FROM threads
                    WHERE board_id = (SELECT id FROM boards WHERE board_key = ?)
                    AND archived = 0
                    ORDER BY last_modified_at DESC
                    LIMIT 1000000 OFFSET ?
                ) AS tmp
            )
            "#,
            board_key,
            target_count,
        )
        .execute(&self.0)
        .await?;

        Ok(())
    }
}
