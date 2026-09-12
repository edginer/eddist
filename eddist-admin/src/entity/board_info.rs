use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "boards_info")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub local_rules: String,
    pub base_thread_creation_span_sec: i32,
    pub base_response_creation_span_sec: i32,
    pub max_thread_name_byte_length: i32,
    pub max_author_name_byte_length: i32,
    pub max_email_byte_length: i32,
    pub max_response_body_byte_length: i32,
    pub max_response_body_lines: i32,
    pub threads_archive_cron: Option<String>,
    pub threads_archive_trigger_thread_count: Option<i32>,
    pub read_only: bool,
    pub created_at: DateTime,
    pub updated_at: DateTime,
    pub force_metadent_type: Option<String>,
    pub enable_1001_message: bool,
    pub custom_1001_message: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
