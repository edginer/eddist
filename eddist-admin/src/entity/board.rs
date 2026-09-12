use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "boards")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    pub board_key: String,
    pub default_name: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_one = "super::board_info::Entity")]
    BoardInfo,
    #[sea_orm(has_many = "super::board_cap::Entity")]
    BoardCap,
    #[sea_orm(has_many = "super::board_ng_word::Entity")]
    BoardNgWord,
    #[sea_orm(has_many = "super::thread::Entity")]
    Thread,
    #[sea_orm(has_many = "super::response::Entity")]
    Response,
}

impl Related<super::board_info::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BoardInfo.def()
    }
}

impl Related<super::board_cap::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BoardCap.def()
    }
}

impl Related<super::board_ng_word::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BoardNgWord.def()
    }
}

impl Related<super::thread::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Thread.def()
    }
}

impl Related<super::response::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Response.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
