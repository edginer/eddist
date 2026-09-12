use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "boards_caps")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub board_id: Uuid,
    pub cap_id: Uuid,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::board::Entity",
        from = "Column::BoardId",
        to = "super::board::Column::Id"
    )]
    Board,
    #[sea_orm(
        belongs_to = "super::cap::Entity",
        from = "Column::CapId",
        to = "super::cap::Column::Id"
    )]
    Cap,
}

impl Related<super::board::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Board.def()
    }
}

impl Related<super::cap::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Cap.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
