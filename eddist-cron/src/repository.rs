use chrono::{TimeZone, Utc};
use sqlx::{types::Json, MySqlPool};
use uuid::Uuid;

use eddist_core::domain::{client_info::ClientInfo, res::ResView};

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
                b.default_name AS default_name,
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
                    ORDER BY last_modified_at ASC
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

    pub async fn get_threads_with_archive_converted(
        &self,
        board_key: &str,
        is_archive_converted: bool,
    ) -> anyhow::Result<Vec<(String, u64, Uuid)>> {
        struct Thread {
            title: String,
            thread_number: i64,
            id: Uuid,
        }

        let threads = sqlx::query_as!(
            Thread,
            r#"
            SELECT
                title,
                thread_number,
                id AS "id: Uuid"
            FROM
                threads
            WHERE
                board_id = (SELECT id FROM boards WHERE board_key = ?)
            AND
                active = 0
            AND
                archived = 1
            AND
                archive_converted = ?
            "#,
            board_key,
            is_archive_converted,
        )
        .fetch_all(&self.0)
        .await?;
        Ok(threads
            .into_iter()
            .map(|t| (t.title, t.thread_number as u64, t.id))
            .collect())
    }

    pub async fn update_archive_converted(&self, thread_id: Uuid) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
            UPDATE threads SET archive_converted = 1 WHERE id = ?
            "#,
            thread_id,
        )
        .execute(&self.0)
        .await?;
        Ok(())
    }

    pub async fn get_thread_responses(
        &self,
        thread_id: Uuid,
    ) -> anyhow::Result<Vec<(ResView, ClientInfo, Uuid)>> {
        let thread_id = Vec::<u8>::from(thread_id);

        struct Res {
            author_name: String,
            mail: String,
            body: String,
            created_at: chrono::NaiveDateTime,
            author_id: String,
            is_abone: i8,
            authed_token_id: Uuid,
            client_info: Json<ClientInfo>,
        }

        let responses = sqlx::query_as!(
            Res,
            r#"
            SELECT
                author_name,
                mail,
                body,
                created_at,
                author_id,
                is_abone,
                authed_token_id AS "authed_token_id: Uuid",
                client_info AS "client_info: Json<ClientInfo>"
            FROM
                responses
            WHERE
                thread_id = ?
            ORDER BY res_order, id
            "#,
            thread_id,
        )
        .fetch_all(&self.0)
        .await?;

        Ok(responses
            .into_iter()
            .map(|r| {
                (
                    ResView {
                        author_name: r.author_name,
                        mail: r.mail,
                        body: r.body,
                        created_at: Utc.from_utc_datetime(&r.created_at),
                        author_id: r.author_id,
                        is_abone: r.is_abone == 1,
                    },
                    r.client_info.0,
                    r.authed_token_id,
                )
            })
            .collect::<Vec<_>>())
    }

    pub async fn archive_thread_and_responses(&self, thread_id: Uuid) -> anyhow::Result<()> {
        let thread_id = Vec::<u8>::from(thread_id);
        let mut tx = self.0.begin().await?;

        sqlx::query!(
            r#"
            INSERT INTO archived_threads 
                (
                    id,
                    board_id,
                    thread_number,
                    last_modified_at,
                    sage_last_modified_at,
                    title,
                    authed_token_id,
                    metadent,
                    response_count,
                    no_pool,
                    active,
                    archived
                ) SELECT
                    id,
                    board_id,
                    thread_number,
                    last_modified_at,
                    sage_last_modified_at,
                    title,
                    authed_token_id,
                    metadent,
                    response_count,
                    no_pool,
                    active,
                    archived
                FROM
                    threads
                WHERE
                    id = ?
            "#,
            thread_id,
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
            INSERT INTO archived_responses 
                (
                    id,
                    author_name,
                    mail,
                    body,
                    created_at,
                    author_id,
                    ip_addr,
                    authed_token_id,
                    board_id,
                    thread_id,
                    is_abone,
                    res_order,
                    client_info
                ) SELECT
                    id,
                    author_name,
                    mail,
                    body,
                    created_at,
                    author_id,
                    ip_addr,
                    authed_token_id,
                    board_id,
                    thread_id,
                    is_abone,
                    res_order,
                    client_info
                FROM
                    responses
                WHERE
                    thread_id = ?
            "#,
            thread_id,
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"
            DELETE FROM threads WHERE id = ?
            "#,
            thread_id,
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SelectionBoardInfo {
    pub board_key: String,
    pub default_name: String,
    pub threads_archive_cron: Option<String>,
    pub threads_archive_trigger_thread_count: Option<i32>,
}
