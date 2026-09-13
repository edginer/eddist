use crate::entity::board;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect,
};
use uuid::Uuid;

pub(crate) async fn board_id_by_key<C: ConnectionTrait>(
    db: &C,
    board_key: &str,
) -> anyhow::Result<Option<Uuid>> {
    Ok(board::Entity::find()
        .filter(board::Column::BoardKey.eq(board_key))
        .one(db)
        .await?
        .map(|model| model.id))
}

pub(crate) fn as_thread_number(value: u64) -> anyhow::Result<i64> {
    i64::try_from(value).map_err(|_| anyhow::anyhow!("thread number is too large: {value}"))
}

pub(crate) fn as_thread_numbers(values: impl IntoIterator<Item = u64>) -> anyhow::Result<Vec<i64>> {
    values.into_iter().map(as_thread_number).collect()
}

pub(crate) fn empty_to_none(value: String) -> Option<String> {
    if value.is_empty() { None } else { Some(value) }
}

/// `board_caps` and `board_ng_words` differ only in the column naming the parent row, so
/// both are addressed by passing the parent and board columns explicitly.
pub(crate) async fn board_ids_for<E, C>(
    db: &C,
    parent_column: E::Column,
    parent_id: Uuid,
    board_column: E::Column,
) -> anyhow::Result<Vec<Uuid>>
where
    E: EntityTrait,
    C: ConnectionTrait,
{
    Ok(E::find()
        .select_only()
        .column(board_column)
        .filter(parent_column.eq(parent_id))
        .order_by_asc(board_column)
        .into_tuple::<Uuid>()
        .all(db)
        .await?)
}

pub(crate) async fn replace_board_links<E, C, F>(
    db: &C,
    parent_column: E::Column,
    parent_id: Uuid,
    board_ids: Vec<Uuid>,
    link: F,
) -> anyhow::Result<()>
where
    E: EntityTrait,
    C: ConnectionTrait,
    F: Fn(Uuid) -> E::ActiveModel,
    E::ActiveModel: ActiveModelTrait<Entity = E>,
{
    E::delete_many()
        .filter(parent_column.eq(parent_id))
        .exec(db)
        .await?;
    E::insert_many(board_ids.into_iter().map(link))
        .exec(db)
        .await?;
    Ok(())
}
