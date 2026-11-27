use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "app_credentials")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub authed_token_id: Uuid,
    pub platform: String,
    pub scope_hash: Vec<u8>,
    pub bearer_hash: Vec<u8>,
    pub registration_key_hash: Vec<u8>,
    pub registration_request_hash: Vec<u8>,
    pub registered_at: DateTimeUtc,
    pub verified_at: DateTimeUtc,
    pub reattest_after: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
