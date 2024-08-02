use chrono::Utc;
use sqlx::{query, query_as, FromRow, MySqlPool};
use uuid::Uuid;

use crate::{Board, Res, Thread};

#[async_trait::async_trait]
pub trait AdminBbsRepository: Send + Sync {
    async fn get_boards_by_key(&self, keys: Option<Vec<String>>) -> anyhow::Result<Vec<Board>>;
    async fn get_threads_by_thread_id(
        &self,
        board_key: &str,
        thread_numbers: Option<Vec<u64>>,
    ) -> anyhow::Result<Vec<Thread>>;
    async fn get_reses_by_thread_id(
        &self,
        board_id: Uuid,
        thread_id: Uuid,
    ) -> anyhow::Result<Vec<Res>>;

    async fn update_res(
        &self,
        id: Uuid,
        author_name: Option<String>,
        mail: Option<String>,
        body: Option<String>,
        is_abone: Option<bool>,
    ) -> anyhow::Result<Res>;

    async fn delete_authed_token(&self, id: Uuid) -> anyhow::Result<()>;
    async fn delete_authed_token_by_origin_ip(&self, id: Uuid) -> anyhow::Result<()>;
}

pub struct AdminBbsRepositoryImpl(MySqlPool);

impl AdminBbsRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self(pool)
    }
}

#[derive(Debug, FromRow)]
pub struct SelectionBoardWithThreadCount {
    pub id: Vec<u8>,
    pub name: String,
    pub board_key: String,
    pub local_rule: String,
    pub default_name: String,
    pub thread_count: i64,
}

#[derive(Debug, FromRow)]
pub struct SelectionThread {
    pub id: Vec<u8>,
    pub board_id: Vec<u8>,
    pub thread_number: i64,
    pub last_modified_at: chrono::DateTime<Utc>,
    pub sage_last_modified_at: chrono::DateTime<Utc>,
    pub title: String,
    pub authed_token_id: Vec<u8>,
    pub metadent: String,
    pub response_count: i32,
    pub no_pool: bool,
    pub archived: bool,
    pub active: bool,
}

#[derive(Debug)]
pub struct SelectionRes {
    pub id: Vec<u8>,
    pub author_name: String,
    pub mail: String,
    pub body: String,
    pub created_at: chrono::DateTime<Utc>,
    pub author_id: String,
    pub ip_addr: String,
    pub authed_token_id: Vec<u8>,
    pub board_id: Vec<u8>,
    pub thread_id: Vec<u8>,
    pub is_abone: i8,
    pub res_order: i32,
}

#[async_trait::async_trait]
impl AdminBbsRepository for AdminBbsRepositoryImpl {
    async fn get_boards_by_key(&self, keys: Option<Vec<String>>) -> anyhow::Result<Vec<Board>> {
        let pool = &self.0;
        let key_lists = if let Some(keys) = &keys {
            let mut initial = "WHERE board_key IN (".to_string();
            initial.push_str(&keys.iter().map(|_| "?").collect::<Vec<_>>().join(", "));
            initial.push(')');
            initial
        } else {
            "".to_string()
        };

        let query = format!(
            r#"
            SELECT
                id,
                name,
                board_key,
                local_rule,
                default_name,
                (
                    SELECT
                        COUNT(*)
                    FROM
                        threads
                    WHERE
                        board_id = boards.id
                ) AS thread_count
            FROM
                boards
            {key_lists}
            "#
        );

        let mut query = sqlx::query_as::<_, SelectionBoardWithThreadCount>(&query);

        if let Some(keys) = keys {
            for key in keys {
                query = query.bind(key);
            }
        }

        let selected_boards = query.fetch_all(pool).await?;

        Ok(selected_boards
            .into_iter()
            .map(|board| Board {
                id: Uuid::from_slice(&board.id).unwrap().to_string().into(),
                name: board.name,
                board_key: board.board_key,
                local_rule: board.local_rule,
                default_name: board.default_name,
                thread_count: board.thread_count,
            })
            .collect())
    }

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
            .map(|thread| Thread {
                id: Uuid::from_slice(&thread.id).unwrap().to_string().into(),
                board_id: Uuid::from_slice(&thread.board_id)
                    .unwrap()
                    .to_string()
                    .into(),
                thread_number: thread.thread_number as u64,
                last_modified: thread.last_modified_at,
                sage_last_modified: thread.sage_last_modified_at,
                title: thread.title,
                authed_token_id: Uuid::from_slice(&thread.authed_token_id)
                    .unwrap()
                    .to_string()
                    .into(),
                metadent: thread.metadent,
                response_count: thread.response_count as u32,
                no_pool: thread.no_pool,
                archived: thread.archived,
                active: thread.active,
            })
            .collect())
    }

    async fn get_reses_by_thread_id(
        &self,
        board_id: Uuid,
        thread_id: Uuid,
    ) -> anyhow::Result<Vec<Res>> {
        let pool = &self.0;

        let query = query_as!(
            SelectionRes,
            r#"
            SELECT
                *
            FROM
                responses
            WHERE
                board_id = ?
                AND thread_id = ?
            ORDER BY
                res_order ASC
            "#,
            board_id.as_bytes().to_vec(),
            thread_id.as_bytes().to_vec()
        );

        let selected_reses = query.fetch_all(pool).await?;

        Ok(selected_reses
            .into_iter()
            .map(|res| Res {
                id: Uuid::from_slice(&res.id).unwrap().to_string().into(),
                author_name: Some(res.author_name),
                mail: Some(res.mail),
                body: res.body,
                created_at: res.created_at,
                author_id: res.author_id,
                ip_addr: res.ip_addr,
                authed_token_id: Uuid::from_slice(&res.authed_token_id)
                    .unwrap()
                    .to_string()
                    .into(),
                board_id: Uuid::from_slice(&res.board_id).unwrap().to_string().into(),
                thread_id: Uuid::from_slice(&res.thread_id).unwrap().to_string().into(),
                is_abone: res.is_abone != 0,
                res_order: res.res_order,
            })
            .collect())
    }

    async fn update_res(
        &self,
        id: Uuid,
        author_name: Option<String>,
        mail: Option<String>,
        body: Option<String>,
        is_abone: Option<bool>,
    ) -> anyhow::Result<Res> {
        let pool = &self.0;

        let mut sets = Vec::new();
        let mut values = Vec::new();

        if let Some(author_name) = author_name {
            sets.push("author_name = ?");
            values.push(author_name);
        }
        if let Some(mail) = mail {
            sets.push("mail = ?");
            values.push(mail);
        }
        if let Some(body) = body {
            sets.push("body = ?");
            values.push(body);
        }
        if is_abone.is_some() {
            sets.push("is_abone = ?");
        }

        let query = format!(
            r#"
            UPDATE
                responses
            SET
                {}
            WHERE
                id = ?
            "#,
            sets.join(", ")
        );

        let mut query = sqlx::query(&query);
        for v in values {
            query = query.bind(v);
        }
        if let Some(is_abone) = is_abone {
            query = query.bind(is_abone);
        }
        let query = query.bind(id.as_bytes().to_vec());

        query.execute(pool).await?;

        let res = query_as!(
            SelectionRes,
            r#"
            SELECT
                *
            FROM
                responses
            WHERE
                id = ?
            "#,
            id.as_bytes().to_vec()
        )
        .fetch_one(pool)
        .await?;

        Ok(Res {
            id: Uuid::from_slice(&res.id).unwrap().to_string().into(),
            author_name: Some(res.author_name),
            mail: Some(res.mail),
            body: res.body,
            created_at: res.created_at,
            author_id: res.author_id,
            ip_addr: res.ip_addr,
            authed_token_id: Uuid::from_slice(&res.authed_token_id)
                .unwrap()
                .to_string()
                .into(),
            board_id: Uuid::from_slice(&res.board_id).unwrap().to_string().into(),
            thread_id: Uuid::from_slice(&res.thread_id).unwrap().to_string().into(),
            is_abone: res.is_abone != 0,
            res_order: res.res_order,
        })
    }

    async fn delete_authed_token(&self, id: Uuid) -> anyhow::Result<()> {
        let query = query!(
            r#"
            UPDATE
                authed_tokens
            SET
                validity = 0
            WHERE
                id = ?
        "#,
            id.as_bytes().to_vec(),
        );

        query.execute(&self.0).await?;

        Ok(())
    }
    async fn delete_authed_token_by_origin_ip(&self, id: Uuid) -> anyhow::Result<()> {
        let query = query!(
            r#"
            UPDATE
                authed_tokens
            SET
                validity = 0
            WHERE
                id IN (
                    SELECT id FROM (
                        SELECT
                            id
                        FROM
                            authed_tokens
                        WHERE
                            origin_ip = ?
                    ) tmp      
                )
        "#,
            id.as_bytes().to_vec(),
        );

        query.execute(&self.0).await?;

        Ok(())
    }
}
