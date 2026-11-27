use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "app_attest_keys")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub authed_token_id: Uuid,
    pub key_id: Vec<u8>,
    pub public_key: Vec<u8>,
    pub counter: u32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
