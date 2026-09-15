use crate::entity::board;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
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
