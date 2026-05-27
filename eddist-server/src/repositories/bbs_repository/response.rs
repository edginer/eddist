use chrono::{NaiveDateTime, TimeZone, Utc};
use eddist_core::domain::pubsub_repository::CreatingRes;
use eddist_core::domain::res::ResView;
use sqlx::query;
use uuid::Uuid;

use super::BbsRepositoryImpl;

#[async_trait::async_trait]
pub trait ResponseRepository: Send + Sync + 'static {
    async fn get_responses(&self, thread_id: Uuid) -> anyhow::Result<Vec<ResView>>;
    async fn create_response(&self, res: CreatingRes) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
impl ResponseRepository for BbsRepositoryImpl {
    async fn get_responses(&self, thread_id: Uuid) -> anyhow::Result<Vec<ResView>> {
        let responses = sqlx::query_as!(
            SelectionRes,
            r#"SELECT
                author_name,
                mail,
                body,
                created_at,
                author_id,
                is_abone AS "is_abone: bool"
            FROM responses WHERE thread_id = ?
            ORDER BY res_order, id"#,
            thread_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(responses
            .into_iter()
            .map(|x| ResView {
                author_name: x.author_name,
                mail: x.mail,
                body: x.body,
                created_at: Utc.from_utc_datetime(&x.created_at),
                author_id: x.author_id,
                is_abone: x.is_abone,
            })
            .collect())
    }

    async fn create_response(&self, res: CreatingRes) -> anyhow::Result<()> {
        let (res_id, th_id, board_id) = (res.id, res.thread_id, res.board_id);
        let client_info_json = serde_json::to_string(&res.client_info)?;

        let th_query = query!(
            "UPDATE threads SET
                last_modified_at = ?,
                response_count = response_count + 1,
                sage_last_modified_at = (
                    CASE
                        WHEN ? THEN sage_last_modified_at
                        ELSE ?
                    END
                ),
                active = (
                    CASE
                        WHEN response_count >= 1000 THEN 0
                        ELSE 1
                    END
                )
            WHERE id = ?
        ",
            res.created_at,
            res.is_sage,
            res.created_at,
            th_id,
        );

        let res_query = query!(
            r"
            INSERT INTO responses
                (
                    id,
                    author_name,
                    mail,
                    author_id,
                    body,
                    thread_id,
                    board_id,
                    ip_addr,
                    authed_token_id,
                    created_at,
                    client_info,
                    res_order
                )
                VALUES
                (
                    ?, ?, ?, ?, ?,
                    ?, ?, ?, ?, ?,
                    ?, ?
                )",
            res_id,
            res.name,
            res.mail,
            res.author_ch5id,
            res.body,
            th_id,
            board_id,
            res.ip_addr,
            res.authed_token_id,
            res.created_at,
            client_info_json,
            res.res_order,
        );

        let mut tx = self.pool.begin().await?;
        th_query.execute(&mut *tx).await?;
        res_query.execute(&mut *tx).await?;
        tx.commit().await?;

        Ok(())
    }
}

#[derive(Debug)]
struct SelectionRes {
    author_name: String,
    mail: String,
    body: String,
    created_at: NaiveDateTime,
    author_id: String,
    is_abone: bool,
}
