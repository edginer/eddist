use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_name: String,
    pub enabled: bool,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::user_authed_token::Entity")]
    UserAuthedToken,
    #[sea_orm(has_many = "super::user_idp_binding::Entity")]
    UserIdpBinding,
}

impl Related<super::user_authed_token::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserAuthedToken.def()
    }
}

impl Related<super::user_idp_binding::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserIdpBinding.def()
    }
}

impl Related<super::authed_token::Entity> for Entity {
    fn to() -> RelationDef {
        super::user_authed_token::Relation::AuthedToken.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::user_authed_token::Relation::User.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
