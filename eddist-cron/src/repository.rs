use chrono::{TimeZone, Utc};
use eddist_core::domain::{client_info::ClientInfo, res::ResView};
use eddist_entity::{archived_response, archived_thread, board, board_info, response, thread};
use sea_orm::sea_query::{Expr, Query};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, QueryTrait, TransactionTrait,
};
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct Repository(DatabaseConnection);

impl Repository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self(db)
    }
}

async fn board_id_by_key<C: ConnectionTrait>(
    db: &C,
    board_key: &str,
) -> anyhow::Result<Option<Uuid>> {
    Ok(board::Entity::find()
        .filter(board::Column::BoardKey.eq(board_key))
        .one(db)
        .await?
        .map(|model| model.id))
}

fn as_thread_number(value: i64) -> anyhow::Result<u64> {
    u64::try_from(value).map_err(|_| anyhow::anyhow!("negative thread number: {value}"))
}

fn as_i64_thread_number(value: u64) -> anyhow::Result<i64> {
    i64::try_from(value).map_err(|_| anyhow::anyhow!("thread number is too large: {value}"))
}

fn into_response(model: response::Model) -> anyhow::Result<(ResView, ClientInfo, Uuid)> {
    let response::Model {
        author_name,
        mail,
        body,
        created_at,
        author_id,
        is_abone,
        is_abone_keep_id,
        authed_token_id,
        client_info,
        ..
    } = model;
    let client_info = serde_json::from_value::<ClientInfo>(client_info)?;

    Ok((
        ResView {
            author_name,
            mail,
            body,
            created_at: Utc.from_utc_datetime(&created_at),
            author_id,
            is_abone,
            is_abone_keep_id,
        },
        client_info,
        authed_token_id,
    ))
}

impl Repository {
    pub async fn get_all_boards_info(&self) -> anyhow::Result<Vec<SelectionBoardInfo>> {
        let boards = board::Entity::find()
            .find_also_related(board_info::Entity)
            .all(&self.0)
            .await?;

        Ok(boards
            .into_iter()
            .filter_map(|(board, board_info)| {
                board_info.map(|board_info| SelectionBoardInfo {
                    board_id: board.id,
                    board_key: board.board_key,
                    default_name: board.default_name,
                    threads_archive_cron: board_info.threads_archive_cron,
                    threads_archive_trigger_thread_count: board_info
                        .threads_archive_trigger_thread_count,
                    enable_1001_message: board_info.enable_1001_message,
                    custom_1001_message: board_info.custom_1001_message,
                })
            })
            .collect())
    }

    pub async fn get_inactive_thread_numbers_for_board(
        &self,
        board_key: &str,
    ) -> anyhow::Result<Vec<u64>> {
        let Some(board_id) = board_id_by_key(&self.0, board_key).await? else {
            return Ok(Vec::new());
        };

        let numbers = thread::Entity::find()
            .select_only()
            .column(thread::Column::ThreadNumber)
            .filter(thread::Column::BoardId.eq(board_id))
            .filter(thread::Column::Active.eq(false))
            .into_tuple::<i64>()
            .all(&self.0)
            .await?;

        numbers.into_iter().map(as_thread_number).collect()
    }

    pub async fn update_threads_to_inactive(
        &self,
        board_key: &str,
        max_thread_count: u32,
    ) -> anyhow::Result<()> {
        thread::Entity::update_many()
            .col_expr(thread::Column::Archived, Expr::value(true))
            .filter(thread::Column::Active.eq(false))
            .exec(&self.0)
            .await?;

        let Some(board_id) = board_id_by_key(&self.0, board_key).await? else {
            return Ok(());
        };

        let ids = thread::Entity::find()
            .select_only()
            .column(thread::Column::Id)
            .filter(thread::Column::BoardId.eq(board_id))
            .filter(thread::Column::Archived.eq(false))
            .order_by_desc(thread::Column::LastModifiedAt)
            .limit(1_000_000)
            .offset(u64::from(max_thread_count))
            .into_tuple::<Uuid>()
            .all(&self.0)
            .await?;

        if ids.is_empty() {
            return Ok(());
        }

        thread::Entity::update_many()
            .col_expr(thread::Column::Archived, Expr::value(true))
            .col_expr(thread::Column::Active, Expr::value(false))
            .filter(thread::Column::Id.is_in(ids))
            .exec(&self.0)
            .await?;

        Ok(())
    }

    pub async fn get_threads_with_archive_converted(
        &self,
        board_key: &str,
        is_archive_converted: bool,
    ) -> anyhow::Result<Vec<(String, u64, Uuid, chrono::NaiveDateTime)>> {
        let Some(board_id) = board_id_by_key(&self.0, board_key).await? else {
            return Ok(Vec::new());
        };

        let threads = thread::Entity::find()
            .filter(thread::Column::BoardId.eq(board_id))
            .filter(thread::Column::Active.eq(false))
            .filter(thread::Column::Archived.eq(true))
            .filter(thread::Column::ArchiveConverted.eq(is_archive_converted))
            .all(&self.0)
            .await?;

        threads
            .into_iter()
            .map(|thread| {
                Ok((
                    thread.title,
                    as_thread_number(thread.thread_number)?,
                    thread.id,
                    thread.last_modified_at,
                ))
            })
            .collect()
    }

    pub async fn get_archived_threads(
        &self,
        board_key: &str,
        start: u64,
        end: u64,
    ) -> anyhow::Result<Vec<(String, u64, Uuid)>> {
        let Some(board_id) = board_id_by_key(&self.0, board_key).await? else {
            return Ok(Vec::new());
        };
        let start = as_i64_thread_number(start)?;
        let end = as_i64_thread_number(end)?;

        let threads = archived_thread::Entity::find()
            .filter(archived_thread::Column::BoardId.eq(board_id))
            .filter(archived_thread::Column::ThreadNumber.between(start, end))
            .all(&self.0)
            .await?;

        threads
            .into_iter()
            .map(|thread| {
                Ok((
                    thread.title,
                    as_thread_number(thread.thread_number)?,
                    thread.id,
                ))
            })
            .collect()
    }

    pub async fn update_archive_converted(&self, thread_id: Uuid) -> anyhow::Result<()> {
        thread::Entity::update_many()
            .col_expr(thread::Column::ArchiveConverted, Expr::value(true))
            .filter(thread::Column::Id.eq(thread_id))
            .exec(&self.0)
            .await?;
        Ok(())
    }

    pub async fn get_thread_responses(
        &self,
        thread_id: Uuid,
    ) -> anyhow::Result<Vec<(ResView, ClientInfo, Uuid)>> {
        response::Entity::find()
            .filter(response::Column::ThreadId.eq(thread_id))
            .order_by_asc(response::Column::ResOrder)
            .order_by_asc(response::Column::Id)
            .all(&self.0)
            .await?
            .into_iter()
            .map(into_response)
            .collect()
    }

    pub async fn get_archived_thread_responses(
        &self,
        thread_id: Uuid,
    ) -> anyhow::Result<Vec<(ResView, ClientInfo, Uuid)>> {
        // `archived_responses` has no `is_abone_keep_id` column. Reporting false is safe because
        // this only feeds `backfill-convert`, which regenerates a dat solely when the S3 object is
        // missing, so an already-published keep-id line is never overwritten.
        archived_response::Entity::find()
            .filter(archived_response::Column::ThreadId.eq(thread_id))
            .column_as(Expr::value(false), "is_abone_keep_id")
            .order_by_asc(archived_response::Column::ResOrder)
            .order_by_asc(archived_response::Column::Id)
            .into_model::<response::Model>()
            .all(&self.0)
            .await?
            .into_iter()
            .map(into_response)
            .collect()
    }

    pub async fn archive_thread_and_responses(&self, thread_id: Uuid) -> anyhow::Result<()> {
        let tx = self.0.begin().await?;

        // Blocks a late response insert (FK check on the thread row) so it cannot be cascaded
        // away by the delete below without being archived.
        let locked = thread::Entity::find_by_id(thread_id)
            .select_only()
            .column(thread::Column::Id)
            .lock_exclusive()
            .into_tuple::<Uuid>()
            .one(&tx)
            .await?;
        if locked.is_none() {
            return Ok(());
        }

        let mut insert_thread = Query::insert();
        insert_thread
            .into_table(archived_thread::Entity)
            .columns([
                archived_thread::Column::Id,
                archived_thread::Column::BoardId,
                archived_thread::Column::ThreadNumber,
                archived_thread::Column::LastModifiedAt,
                archived_thread::Column::SageLastModifiedAt,
                archived_thread::Column::Title,
                archived_thread::Column::AuthedTokenId,
                archived_thread::Column::Metadent,
                archived_thread::Column::ResponseCount,
                archived_thread::Column::NoPool,
                archived_thread::Column::Active,
                archived_thread::Column::Archived,
            ])
            .select_from(
                thread::Entity::find()
                    .select_only()
                    .columns([
                        thread::Column::Id,
                        thread::Column::BoardId,
                        thread::Column::ThreadNumber,
                        thread::Column::LastModifiedAt,
                        thread::Column::SageLastModifiedAt,
                        thread::Column::Title,
                        thread::Column::AuthedTokenId,
                        thread::Column::Metadent,
                        thread::Column::ResponseCount,
                        thread::Column::NoPool,
                        thread::Column::Active,
                        thread::Column::Archived,
                    ])
                    .filter(thread::Column::Id.eq(thread_id))
                    .into_query(),
            )?;
        tx.execute(&insert_thread).await?;

        let mut insert_responses = Query::insert();
        insert_responses
            .into_table(archived_response::Entity)
            .columns([
                archived_response::Column::Id,
                archived_response::Column::AuthorName,
                archived_response::Column::Mail,
                archived_response::Column::Body,
                archived_response::Column::CreatedAt,
                archived_response::Column::AuthorId,
                archived_response::Column::IpAddr,
                archived_response::Column::AuthedTokenId,
                archived_response::Column::BoardId,
                archived_response::Column::ThreadId,
                archived_response::Column::IsAbone,
                archived_response::Column::ResOrder,
                archived_response::Column::ClientInfo,
            ])
            .select_from(
                response::Entity::find()
                    .select_only()
                    .columns([
                        response::Column::Id,
                        response::Column::AuthorName,
                        response::Column::Mail,
                        response::Column::Body,
                        response::Column::CreatedAt,
                        response::Column::AuthorId,
                        response::Column::IpAddr,
                        response::Column::AuthedTokenId,
                        response::Column::BoardId,
                        response::Column::ThreadId,
                        response::Column::IsAbone,
                        response::Column::ResOrder,
                        response::Column::ClientInfo,
                    ])
                    .filter(response::Column::ThreadId.eq(thread_id))
                    .into_query(),
            )?;
        tx.execute(&insert_responses).await?;

        thread::Entity::delete_by_id(thread_id).exec(&tx).await?;
        tx.commit().await?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SelectionBoardInfo {
    pub board_id: Uuid,
    pub board_key: String,
    pub default_name: String,
    pub threads_archive_cron: Option<String>,
    pub threads_archive_trigger_thread_count: Option<i32>,
    pub enable_1001_message: bool,
    pub custom_1001_message: Option<String>,
}
