use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "archived_threads")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub board_id: Uuid,
    pub thread_number: i64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub last_modified_at: DateTime,
    pub sage_last_modified_at: DateTime,
    pub title: String,
    pub authed_token_id: Uuid,
    pub metadent: String,
    pub response_count: i32,
    pub no_pool: bool,
    pub active: bool,
    pub archived: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
