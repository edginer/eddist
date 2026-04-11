use chrono::{TimeZone, Utc};
use eddist_core::domain::client_info::ClientInfo;
#[cfg(not(feature = "backend-postgres"))]
use sqlx::{MySqlPool, query_as, types::Json};
#[cfg(feature = "backend-postgres")]
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::Res;

#[cfg(not(feature = "backend-postgres"))]
use super::admin_bbs_repository::SelectionRes;
#[cfg(feature = "backend-postgres")]
use super::admin_bbs_repository::SelectionResPg;

#[async_trait::async_trait]
pub trait AdminResponseRepository: Send + Sync {
    async fn get_reses_by_thread_id(
        &self,
        board_key: &str,
        thread_number: u64,
    ) -> anyhow::Result<Vec<Res>>;
    async fn get_archived_reses_by_thread_id(
        &self,
        board_key: &str,
        thread_number: u64,
    ) -> anyhow::Result<Vec<Res>>;
    async fn get_res(
        &self,
        res_id: Uuid,
    ) -> anyhow::Result<(Res, String, String, u64, Option<String>)>;
    async fn update_res(
        &self,
        id: Uuid,
        author_name: Option<String>,
        mail: Option<String>,
        body: Option<String>,
        is_abone: Option<bool>,
    ) -> anyhow::Result<Res>;
}

#[cfg(not(feature = "backend-postgres"))]
#[derive(Clone)]
pub struct AdminResponseRepositoryImpl(pub(crate) MySqlPool);

#[cfg(not(feature = "backend-postgres"))]
impl AdminResponseRepositoryImpl {
    pub fn new(pool: MySqlPool) -> Self {
        Self(pool)
    }
}

#[cfg(not(feature = "backend-postgres"))]
fn selection_res_to_res(res: SelectionRes) -> Res {
    Res {
        id: Uuid::from_slice(&res.id).unwrap(),
        author_name: Some(res.author_name),
        mail: Some(res.mail),
        body: res.body,
        created_at: Utc.from_utc_datetime(&res.created_at),
        author_id: res.author_id,
        ip_addr: res.ip_addr,
        authed_token_id: Uuid::from_slice(&res.authed_token_id).unwrap(),
        board_id: Uuid::from_slice(&res.board_id).unwrap(),
        thread_id: Uuid::from_slice(&res.thread_id).unwrap(),
        is_abone: res.is_abone != 0,
        client_info: res.client_info.0.into(),
        res_order: res.res_order,
    }
}

#[cfg(not(feature = "backend-postgres"))]
#[async_trait::async_trait]
impl AdminResponseRepository for AdminResponseRepositoryImpl {
    async fn get_reses_by_thread_id(
        &self,
        board_key: &str,
        thread_number: u64,
    ) -> anyhow::Result<Vec<Res>> {
        let pool = &self.0;

        let query = query_as!(
            SelectionRes,
            r#"
            SELECT
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
                client_info AS "client_info!: Json<ClientInfo>",
                res_order
            FROM
                responses
            WHERE
                thread_id = (
                    SELECT
                        id
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
                    AND
                        thread_number = ?
                )
            ORDER BY
                res_order ASC
            "#,
            board_key,
            thread_number
        );

        let selected_reses = query.fetch_all(pool).await?;
        Ok(selected_reses
            .into_iter()
            .map(selection_res_to_res)
            .collect())
    }

    async fn get_archived_reses_by_thread_id(
        &self,
        board_key: &str,
        thread_number: u64,
    ) -> anyhow::Result<Vec<Res>> {
        let pool = &self.0;

        let query = query_as!(
            SelectionRes,
            r#"
            SELECT
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
                client_info AS "client_info!: Json<ClientInfo>",
                res_order
            FROM
                archived_responses
            WHERE
                thread_id = (
                    SELECT
                        id
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
                    AND
                        thread_number = ?
                )
            ORDER BY
                res_order ASC
            "#,
            board_key,
            thread_number
        );

        let selected_reses = query.fetch_all(pool).await?;
        Ok(selected_reses
            .into_iter()
            .map(selection_res_to_res)
            .collect())
    }

    async fn get_res(
        &self,
        res_id: Uuid,
    ) -> anyhow::Result<(Res, String, String, u64, Option<String>)> {
        let pool = &self.0;

        let res = query_as!(
            SelectionRes,
            r#"
            SELECT
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
                client_info AS "client_info!: Json<ClientInfo>",
                res_order
            FROM
                responses
            WHERE
                id = ?
            "#,
            res_id.as_bytes().to_vec()
        )
        .fetch_one(pool)
        .await?;

        let res = selection_res_to_res(res);

        struct BoardKeyThreadNumber {
            board_key: String,
            thread_number: u64,
            default_name: String,
            thread_title: Option<String>,
        }

        let board_key = query_as!(
            BoardKeyThreadNumber,
            r#"
            SELECT
                boards.board_key AS "board_key!: String",
                threads.thread_number AS "thread_number!: u64",
                boards.default_name AS "default_name!: String",
                threads.title AS "thread_title: String"
            FROM
                boards
            JOIN threads ON boards.id = threads.board_id
            WHERE
                threads.id = ?
            "#,
            res.thread_id,
        )
        .fetch_one(pool)
        .await?;

        Ok((
            res,
            board_key.default_name,
            board_key.board_key,
            board_key.thread_number,
            board_key.thread_title,
        ))
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
                client_info AS "client_info!: Json<ClientInfo>",
                res_order
            FROM
                responses
            WHERE
                id = ?
            "#,
            id.as_bytes().to_vec()
        )
        .fetch_one(pool)
        .await?;

        Ok(selection_res_to_res(res))
    }
}

#[cfg(feature = "backend-postgres")]
fn selection_res_pg_to_res(res: SelectionResPg) -> Res {
    Res {
        id: res.id,
        author_name: Some(res.author_name),
        mail: Some(res.mail),
        body: res.body,
        created_at: res.created_at,
        author_id: res.author_id,
        ip_addr: res.ip_addr,
        authed_token_id: res.authed_token_id,
        board_id: res.board_id,
        thread_id: res.thread_id,
        is_abone: res.is_abone,
        client_info: res.client_info.0.into(),
        res_order: res.res_order,
    }
}

#[cfg(feature = "backend-postgres")]
#[derive(Clone)]
pub struct AdminResponseRepositoryPgImpl(pub(crate) PgPool);

#[cfg(feature = "backend-postgres")]
impl AdminResponseRepositoryPgImpl {
    pub fn new(pool: PgPool) -> Self {
        Self(pool)
    }
}

#[cfg(feature = "backend-postgres")]
#[async_trait::async_trait]
impl AdminResponseRepository for AdminResponseRepositoryPgImpl {
    async fn get_reses_by_thread_id(
        &self,
        board_key: &str,
        thread_number: u64,
    ) -> anyhow::Result<Vec<Res>> {
        let pool = &self.0;

        let reses = sqlx::query_as::<_, SelectionResPg>(
            r#"
            SELECT
                id, author_name, mail, body, created_at, author_id, ip_addr,
                authed_token_id, board_id, thread_id, is_abone, client_info, res_order
            FROM responses
            WHERE thread_id = (
                SELECT id FROM threads
                WHERE board_id = (SELECT id FROM boards WHERE board_key = $1)
                AND thread_number = $2
            )
            ORDER BY res_order ASC
            "#,
        )
        .bind(board_key)
        .bind(thread_number as i64)
        .fetch_all(pool)
        .await?;

        Ok(reses.into_iter().map(selection_res_pg_to_res).collect())
    }

    async fn get_archived_reses_by_thread_id(
        &self,
        board_key: &str,
        thread_number: u64,
    ) -> anyhow::Result<Vec<Res>> {
        let pool = &self.0;

        let reses = sqlx::query_as::<_, SelectionResPg>(
            r#"
            SELECT
                id, author_name, mail, body, created_at, author_id, ip_addr,
                authed_token_id, board_id, thread_id, is_abone, client_info, res_order
            FROM archived_responses
            WHERE thread_id = (
                SELECT id FROM archived_threads
                WHERE board_id = (SELECT id FROM boards WHERE board_key = $1)
                AND thread_number = $2
            )
            ORDER BY res_order ASC
            "#,
        )
        .bind(board_key)
        .bind(thread_number as i64)
        .fetch_all(pool)
        .await?;

        Ok(reses.into_iter().map(selection_res_pg_to_res).collect())
    }

    async fn get_res(
        &self,
        res_id: Uuid,
    ) -> anyhow::Result<(Res, String, String, u64, Option<String>)> {
        let pool = &self.0;

        let res = sqlx::query_as::<_, SelectionResPg>(
            r#"
            SELECT
                id, author_name, mail, body, created_at, author_id, ip_addr,
                authed_token_id, board_id, thread_id, is_abone, client_info, res_order
            FROM responses
            WHERE id = $1
            "#,
        )
        .bind(res_id)
        .fetch_one(pool)
        .await?;

        let res = selection_res_pg_to_res(res);

        #[derive(sqlx::FromRow)]
        struct BoardKeyThreadNumber {
            board_key: String,
            thread_number: i64,
            default_name: String,
            thread_title: Option<String>,
        }

        let meta = sqlx::query_as::<_, BoardKeyThreadNumber>(
            r#"
            SELECT
                boards.board_key,
                threads.thread_number,
                boards.default_name,
                threads.title AS thread_title
            FROM boards
            JOIN threads ON boards.id = threads.board_id
            WHERE threads.id = $1
            "#,
        )
        .bind(res.thread_id)
        .fetch_one(pool)
        .await?;

        Ok((
            res,
            meta.default_name,
            meta.board_key,
            meta.thread_number as u64,
            meta.thread_title,
        ))
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

        let mut sets: Vec<String> = Vec::new();
        let mut idx = 1usize;

        if author_name.is_some() {
            sets.push(format!("author_name = ${idx}"));
            idx += 1;
        }
        if mail.is_some() {
            sets.push(format!("mail = ${idx}"));
            idx += 1;
        }
        if body.is_some() {
            sets.push(format!("body = ${idx}"));
            idx += 1;
        }
        if is_abone.is_some() {
            sets.push(format!("is_abone = ${idx}"));
            idx += 1;
        }

        let sql = format!(
            "UPDATE responses SET {} WHERE id = ${}",
            sets.join(", "),
            idx
        );

        let mut q = sqlx::query(&sql);
        if let Some(ref v) = author_name { q = q.bind(v); }
        if let Some(ref v) = mail { q = q.bind(v); }
        if let Some(ref v) = body { q = q.bind(v); }
        if let Some(v) = is_abone { q = q.bind(v); }
        q = q.bind(id);
        q.execute(pool).await?;

        let res = sqlx::query_as::<_, SelectionResPg>(
            r#"
            SELECT
                id, author_name, mail, body, created_at, author_id, ip_addr,
                authed_token_id, board_id, thread_id, is_abone, client_info, res_order
            FROM responses
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await?;

        Ok(selection_res_pg_to_res(res))
    }
}
