use crate::entity::{archived_response, archived_thread, board, response, thread};
use crate::error::DbResultExt;
use crate::models::Res;
use crate::repository::support::{as_thread_number, board_id_by_key};
use eddist_core::domain::client_info::ClientInfo as CoreClientInfo;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait,
    IntoActiveValue, QueryFilter, QueryOrder, QuerySelect,
};
use uuid::Uuid;

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
        is_abone_keep_id: Option<bool>,
    ) -> anyhow::Result<Res>;
}

#[derive(Clone)]
pub struct AdminResponseRepositoryImpl(pub(crate) DatabaseConnection);

impl AdminResponseRepositoryImpl {
    pub fn new(db: DatabaseConnection) -> Self {
        Self(db)
    }
}

fn into_response(model: response::Model) -> anyhow::Result<Res> {
    let client_info = serde_json::from_value::<CoreClientInfo>(model.client_info)?;
    Ok(Res {
        id: model.id,
        author_name: Some(model.author_name),
        mail: Some(model.mail),
        body: model.body,
        created_at: model.created_at.and_utc(),
        author_id: model.author_id,
        ip_addr: model.ip_addr,
        authed_token_id: model.authed_token_id,
        board_id: model.board_id,
        thread_id: model.thread_id,
        is_abone: model.is_abone,
        is_abone_keep_id: model.is_abone_keep_id,
        client_info: client_info.into(),
        res_order: model.res_order,
    })
}

#[async_trait::async_trait]
impl AdminResponseRepository for AdminResponseRepositoryImpl {
    async fn get_reses_by_thread_id(
        &self,
        board_key: &str,
        thread_number: u64,
    ) -> anyhow::Result<Vec<Res>> {
        let Some(board_id) = board_id_by_key(&self.0, board_key).await? else {
            return Ok(Vec::new());
        };
        let Some(thread) = thread::Entity::find()
            .filter(thread::Column::BoardId.eq(board_id))
            .filter(thread::Column::ThreadNumber.eq(as_thread_number(thread_number)?))
            .one(&self.0)
            .await?
        else {
            return Ok(Vec::new());
        };

        response::Entity::find()
            .filter(response::Column::ThreadId.eq(thread.id))
            .order_by_asc(response::Column::ResOrder)
            .order_by_asc(response::Column::Id)
            .all(&self.0)
            .await?
            .into_iter()
            .map(into_response)
            .collect()
    }

    async fn get_archived_reses_by_thread_id(
        &self,
        board_key: &str,
        thread_number: u64,
    ) -> anyhow::Result<Vec<Res>> {
        let Some(board_id) = board_id_by_key(&self.0, board_key).await? else {
            return Ok(Vec::new());
        };
        let Some(thread) = archived_thread::Entity::find()
            .filter(archived_thread::Column::BoardId.eq(board_id))
            .filter(archived_thread::Column::ThreadNumber.eq(as_thread_number(thread_number)?))
            .one(&self.0)
            .await?
        else {
            return Ok(Vec::new());
        };

        // `archived_responses` has no `is_abone_keep_id`; supplying it as a literal lets
        // archived rows share the live conversion.
        archived_response::Entity::find()
            .filter(archived_response::Column::ThreadId.eq(thread.id))
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

    async fn get_res(
        &self,
        res_id: Uuid,
    ) -> anyhow::Result<(Res, String, String, u64, Option<String>)> {
        let (model, thread) = response::Entity::find_by_id(res_id)
            .find_also_related(thread::Entity)
            .one(&self.0)
            .await?
            .ok_or_else(|| crate::error::ServiceError::NotFound("Response not found".into()))?;
        let res = into_response(model)?;
        let thread =
            thread.ok_or_else(|| anyhow::anyhow!("Thread not found: {}", res.thread_id))?;

        let board = board::Entity::find_by_id(thread.board_id)
            .one(&self.0)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Board not found: {}", thread.board_id))?;
        let thread_number = u64::try_from(thread.thread_number)
            .map_err(|_| anyhow::anyhow!("negative thread number: {}", thread.thread_number))?;

        Ok((
            res,
            board.default_name,
            board.board_key,
            thread_number,
            Some(thread.title),
        ))
    }

    async fn update_res(
        &self,
        id: Uuid,
        author_name: Option<String>,
        mail: Option<String>,
        body: Option<String>,
        is_abone: Option<bool>,
        is_abone_keep_id: Option<bool>,
    ) -> anyhow::Result<Res> {
        let updated = response::ActiveModel {
            id: Set(id),
            author_name: author_name.into_active_value(),
            mail: mail.into_active_value(),
            body: body.into_active_value(),
            is_abone: is_abone.into_active_value(),
            is_abone_keep_id: is_abone_keep_id.into_active_value(),
            ..Default::default()
        }
        .update(&self.0)
        .await
        .or_not_found("Response")?;

        into_response(updated)
    }
}
