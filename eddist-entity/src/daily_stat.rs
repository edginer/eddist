use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "daily_stats")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub date: Date,
    #[sea_orm(primary_key, auto_increment = false)]
    pub board_key: String,
    pub total_responses: i64,
    pub new_threads: i64,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
