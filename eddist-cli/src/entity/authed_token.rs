use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "authed_tokens")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub token: String,
    pub origin_ip: String,
    pub reduced_origin_ip: String,
    pub writing_ua: String,
    pub authed_ua: Option<String>,
    pub auth_code: String,
    pub created_at: DateTime,
    pub authed_at: Option<DateTime>,
    pub validity: bool,
    pub last_wrote_at: Option<DateTime>,
    pub asn_num: i32,
    pub additional_info: Option<Json>,
    pub require_user_registration: bool,
    pub registered_user_id: Option<Uuid>,
    pub require_reauth: bool,
    pub author_id_seed: Vec<u8>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
