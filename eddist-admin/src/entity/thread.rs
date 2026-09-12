use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "threads")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub board_id: Uuid,
    pub thread_number: i64,
    pub last_modified_at: DateTime,
    pub sage_last_modified_at: DateTime,
    pub title: String,
    pub authed_token_id: Uuid,
    pub metadent: String,
    pub response_count: i32,
    pub no_pool: bool,
    pub active: bool,
    pub archived: bool,
    pub archive_converted: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::authed_token::Entity",
        from = "Column::AuthedTokenId",
        to = "super::authed_token::Column::Id"
    )]
    AuthedToken,
    #[sea_orm(
        belongs_to = "super::board::Entity",
        from = "Column::BoardId",
        to = "super::board::Column::Id"
    )]
    Board,
    #[sea_orm(has_many = "super::response::Entity")]
    Response,
}

impl Related<super::authed_token::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AuthedToken.def()
    }
}

impl Related<super::board::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Board.def()
    }
}

impl Related<super::response::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Response.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
