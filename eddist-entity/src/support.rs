use sea_orm::prelude::Uuid;
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QuerySelect};

use crate::board;

pub async fn board_id_by_key<C: ConnectionTrait>(
    db: &C,
    board_key: &str,
) -> Result<Option<Uuid>, DbErr> {
    board::Entity::find()
        .select_only()
        .column(board::Column::Id)
        .filter(board::Column::BoardKey.eq(board_key))
        .into_tuple()
        .one(db)
        .await
}

pub fn thread_number_to_db(value: u64) -> anyhow::Result<i64> {
    i64::try_from(value).map_err(|_| anyhow::anyhow!("thread number is too large: {value}"))
}

pub fn thread_numbers_to_db(values: impl IntoIterator<Item = u64>) -> anyhow::Result<Vec<i64>> {
    values.into_iter().map(thread_number_to_db).collect()
}

pub fn thread_number_from_db(value: i64) -> anyhow::Result<u64> {
    u64::try_from(value).map_err(|_| anyhow::anyhow!("negative thread number: {value}"))
}
