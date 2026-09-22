use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "captcha_configs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    pub provider: String,
    pub site_key: String,
    pub secret: String,
    pub base_url: Option<String>,
    pub widget_form_field_name: Option<String>,
    pub widget_script_url: Option<String>,
    pub widget_html: Option<String>,
    pub widget_script_handler: Option<String>,
    pub capture_fields: Option<Json>,
    pub verification: Option<Json>,
    pub is_active: bool,
    pub display_order: i32,
    pub endpoint_usage: String,
    pub created_at: DateTime,
    pub updated_at: DateTime,
    pub updated_by: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
