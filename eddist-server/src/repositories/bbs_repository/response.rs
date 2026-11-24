use chrono::{NaiveDateTime, TimeZone, Utc};
use eddist_core::domain::pubsub_repository::CreatingRes;
use eddist_core::domain::res::ResView;
use eddist_entity::{db_time, response, thread};
use sea_orm::{
    ActiveValue::Set,
    ColumnTrait, ConnectionTrait, EntityTrait, FromQueryResult, IsolationLevel, QueryFilter,
    QueryOrder, QuerySelect, TransactionTrait,
    sea_query::{Expr, ExprTrait, Query, UpdateStatement},
};
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
        use response::Column;
        let responses = response::Entity::find()
            .select_only()
            .columns([
                Column::AuthorName,
                Column::Mail,
                Column::Body,
                Column::CreatedAt,
                Column::AuthorId,
                Column::IsAbone,
                Column::IsAboneKeepId,
            ])
            .filter(Column::ThreadId.eq(thread_id))
            .order_by_asc(Column::ResOrder)
            .order_by_asc(Column::Id)
            .into_model::<SelectionRes>()
            .all(&self.db)
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
                is_abone_keep_id: x.is_abone_keep_id,
            })
            .collect())
    }

    async fn create_response(&self, res: CreatingRes) -> anyhow::Result<()> {
        let update = thread_response_update(&res);
        let model = response_model(res)?;
        let tx = self
            .db
            .begin_with_config(Some(IsolationLevel::ReadCommitted), None)
            .await?;
        tx.execute(&update).await?;
        response::Entity::insert(model)
            .exec_without_returning(&tx)
            .await?;
        tx.commit().await?;

        Ok(())
    }
}

// MySQL evaluates SET assignments from left to right. Calculate active from the
// old count before incrementing it, so PostgreSQL's old-row evaluation agrees.
fn thread_response_update(res: &CreatingRes) -> UpdateStatement {
    use thread::Column;
    let created_at = db_time::truncate_to_millis(res.created_at.naive_utc());
    Query::update()
        .table(thread::Entity)
        .values([
            (
                Column::Active,
                Expr::case(Expr::col(Column::ResponseCount).add(1).gte(1000), false)
                    .finally(true)
                    .into(),
            ),
            (
                Column::ResponseCount,
                Expr::col(Column::ResponseCount).add(1),
            ),
            (Column::LastModifiedAt, Expr::value(created_at)),
            (
                Column::SageLastModifiedAt,
                Expr::case(
                    Expr::val(res.is_sage),
                    Expr::col(Column::SageLastModifiedAt),
                )
                .finally(created_at)
                .into(),
            ),
        ])
        .and_where(Expr::col(Column::Id).eq(res.thread_id))
        .to_owned()
}

pub(super) fn response_model(res: CreatingRes) -> anyhow::Result<response::ActiveModel> {
    Ok(response::ActiveModel {
        id: Set(res.id),
        author_name: Set(res.name),
        mail: Set(res.mail),
        body: Set(res.body),
        created_at: Set(db_time::truncate_to_millis(res.created_at.naive_utc())),
        author_id: Set(res.author_ch5id),
        ip_addr: Set(res.ip_addr),
        authed_token_id: Set(res.authed_token_id),
        board_id: Set(res.board_id),
        thread_id: Set(res.thread_id),
        client_info: Set(serde_json::to_value(res.client_info)?),
        res_order: Set(res.res_order),
        ..Default::default()
    })
}

#[derive(Debug, FromQueryResult)]
struct SelectionRes {
    author_name: String,
    mail: String,
    body: String,
    created_at: NaiveDateTime,
    author_id: String,
    is_abone: bool,
    is_abone_keep_id: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::sea_query::{MysqlQueryBuilder, PostgresQueryBuilder};

    #[test]
    fn response_update_uses_old_count_before_incrementing_in_both_dialects() {
        let res = CreatingRes {
            id: Uuid::nil(),
            created_at: Utc::now(),
            body: String::new(),
            name: String::new(),
            mail: String::new(),
            author_ch5id: String::new(),
            authed_token_id: Uuid::nil(),
            ip_addr: String::new(),
            thread_id: Uuid::nil(),
            board_id: Uuid::nil(),
            client_info: Default::default(),
            res_order: 1000,
            is_sage: true,
            moderation_result: None,
        };
        let update = thread_response_update(&res);
        for sql in [
            update.to_string(MysqlQueryBuilder),
            update.to_string(PostgresQueryBuilder),
        ] {
            let normalized: String = sql
                .chars()
                .filter(|ch| !matches!(ch, '`' | '"' | ' ' | '(' | ')'))
                .collect();
            assert!(normalized.contains("SETactive=CASEWHENresponse_count+1>=1000THENFALSEELSETRUEEND,response_count=response_count+1,"), "{sql}");
            assert!(
                normalized
                    .contains("sage_last_modified_at=CASEWHENTRUETHENsage_last_modified_atELSE"),
                "{sql}"
            );
        }
    }
}
