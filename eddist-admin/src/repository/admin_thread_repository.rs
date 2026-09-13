use crate::entity::{archived_thread, thread};
use crate::models::Thread;
use crate::repository::support::{as_thread_numbers, board_id_by_key};
use sea_orm::sea_query::Expr;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use uuid::Uuid;

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
        range: (
            Option<chrono::DateTime<chrono::Utc>>,
            Option<chrono::DateTime<chrono::Utc>>,
        ),
        page: u64,
        limit: u64,
    ) -> anyhow::Result<Vec<Thread>>;
    async fn compact_threads(&self, board_key: &str, target_count: u32) -> anyhow::Result<()>;
    async fn archive_threads(&self, board_key: &str, thread_numbers: &[u64]) -> anyhow::Result<()>;
}

#[derive(Clone)]
pub struct AdminThreadRepositoryImpl(DatabaseConnection);

impl AdminThreadRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self(db)
    }
}

fn into_thread(model: thread::Model) -> anyhow::Result<Thread> {
    Ok(Thread {
        id: model.id,
        board_id: model.board_id,
        thread_number: u64::try_from(model.thread_number)
            .map_err(|_| anyhow::anyhow!("negative thread number: {}", model.thread_number))?,
        last_modified: model.last_modified_at.and_utc(),
        sage_last_modified: model.sage_last_modified_at.and_utc(),
        title: model.title,
        authed_token_id: model.authed_token_id,
        metadent: model.metadent,
        response_count: u32::try_from(model.response_count)
            .map_err(|_| anyhow::anyhow!("negative response count: {}", model.response_count))?,
        no_pool: model.no_pool,
        archived: model.archived,
        active: model.active,
    })
}

/// `archived_threads` has no `archive_converted`; supplying it as a literal lets archived
/// rows share the live conversion.
fn archived_as_thread_model(
    query: sea_orm::Select<archived_thread::Entity>,
) -> sea_orm::Selector<sea_orm::SelectModel<thread::Model>> {
    query
        .column_as(Expr::value(false), "archive_converted")
        .into_model::<thread::Model>()
}

#[async_trait::async_trait]
impl AdminThreadRepository for AdminThreadRepositoryImpl {
    async fn get_threads_by_thread_id(
        &self,
        board_key: &str,
        thread_numbers: Option<Vec<u64>>,
    ) -> anyhow::Result<Vec<Thread>> {
        let Some(board_id) = board_id_by_key(&self.0, board_key).await? else {
            return Ok(Vec::new());
        };

        let mut query = thread::Entity::find().filter(thread::Column::BoardId.eq(board_id));
        if let Some(thread_numbers) = thread_numbers {
            query = query
                .filter(thread::Column::ThreadNumber.is_in(as_thread_numbers(thread_numbers)?));
        }

        query
            .all(&self.0)
            .await?
            .into_iter()
            .map(into_thread)
            .collect()
    }

    async fn get_archived_threads_by_thread_id(
        &self,
        board_key: &str,
        thread_numbers: Option<Vec<u64>>,
    ) -> anyhow::Result<Vec<Thread>> {
        let Some(board_id) = board_id_by_key(&self.0, board_key).await? else {
            return Ok(Vec::new());
        };

        let mut query =
            archived_thread::Entity::find().filter(archived_thread::Column::BoardId.eq(board_id));
        if let Some(thread_numbers) = thread_numbers {
            query = query.filter(
                archived_thread::Column::ThreadNumber.is_in(as_thread_numbers(thread_numbers)?),
            );
        }

        archived_as_thread_model(query)
            .all(&self.0)
            .await?
            .into_iter()
            .map(into_thread)
            .collect()
    }

    async fn get_archived_threads_by_filter(
        &self,
        board_key: &str,
        keyword: Option<&str>,
        range: (
            Option<chrono::DateTime<chrono::Utc>>,
            Option<chrono::DateTime<chrono::Utc>>,
        ),
        page: u64,
        limit: u64,
    ) -> anyhow::Result<Vec<Thread>> {
        let Some(board_id) = board_id_by_key(&self.0, board_key).await? else {
            return Ok(Vec::new());
        };

        let mut query =
            archived_thread::Entity::find().filter(archived_thread::Column::BoardId.eq(board_id));
        if let Some(keyword) = keyword {
            query = query.filter(archived_thread::Column::Title.contains(keyword));
        }
        if let (Some(start), Some(end)) = range {
            query = query.filter(
                archived_thread::Column::LastModifiedAt.between(start.naive_utc(), end.naive_utc()),
            );
        }

        archived_as_thread_model(
            query
                .order_by_desc(archived_thread::Column::LastModifiedAt)
                .limit(limit)
                .offset(page.saturating_mul(limit)),
        )
        .all(&self.0)
        .await?
        .into_iter()
        .map(into_thread)
        .collect()
    }

    async fn compact_threads(&self, board_key: &str, target_count: u32) -> anyhow::Result<()> {
        let Some(board_id) = board_id_by_key(&self.0, board_key).await? else {
            return Ok(());
        };

        let ids = thread::Entity::find()
            .select_only()
            .column(thread::Column::Id)
            .filter(thread::Column::BoardId.eq(board_id))
            .filter(thread::Column::Archived.eq(false))
            .order_by_desc(thread::Column::LastModifiedAt)
            .offset(u64::from(target_count))
            // Caps how many threads one call archives; MySQL also rejects OFFSET without LIMIT.
            .limit(1000)
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

    async fn archive_threads(&self, board_key: &str, thread_numbers: &[u64]) -> anyhow::Result<()> {
        if thread_numbers.is_empty() {
            return Ok(());
        }
        let Some(board_id) = board_id_by_key(&self.0, board_key).await? else {
            return Ok(());
        };

        thread::Entity::update_many()
            .col_expr(thread::Column::Archived, Expr::value(true))
            .col_expr(thread::Column::Active, Expr::value(false))
            .filter(thread::Column::BoardId.eq(board_id))
            .filter(thread::Column::Archived.eq(false))
            .filter(
                thread::Column::ThreadNumber
                    .is_in(as_thread_numbers(thread_numbers.iter().copied())?),
            )
            .exec(&self.0)
            .await?;

        Ok(())
    }
}
