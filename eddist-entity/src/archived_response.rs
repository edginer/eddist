use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "archived_responses")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub author_name: String,
    pub mail: String,
    pub body: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub created_at: DateTime,
    pub author_id: String,
    pub ip_addr: String,
    pub authed_token_id: Uuid,
    pub board_id: Uuid,
    pub thread_id: Uuid,
    pub is_abone: bool,
    pub res_order: i32,
    pub client_info: Json,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
