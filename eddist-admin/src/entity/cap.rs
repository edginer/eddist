use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "caps")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub password_hash: String,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::board_cap::Entity")]
    BoardCap,
}

impl Related<super::board_cap::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BoardCap.def()
    }
}

impl Related<super::board::Entity> for Entity {
    fn to() -> RelationDef {
        super::board_cap::Relation::Board.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::board_cap::Relation::Cap.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
