use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "ng_words")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    pub word: String,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::board_ng_word::Entity")]
    BoardNgWord,
}

impl Related<super::board_ng_word::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BoardNgWord.def()
    }
}

impl Related<super::board::Entity> for Entity {
    fn to() -> RelationDef {
        super::board_ng_word::Relation::Board.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::board_ng_word::Relation::NgWord.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
