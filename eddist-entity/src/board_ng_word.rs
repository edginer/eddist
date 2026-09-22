use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "boards_ng_words")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub board_id: Uuid,
    pub ng_word_id: Uuid,
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
        belongs_to = "super::ng_word::Entity",
        from = "Column::NgWordId",
        to = "super::ng_word::Column::Id"
    )]
    NgWord,
}

impl Related<super::board::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Board.def()
    }
}

impl Related<super::ng_word::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::NgWord.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
