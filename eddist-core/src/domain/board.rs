use chrono::NaiveDateTime;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Board {
    pub id: Uuid,
    pub name: String,
    pub board_key: String,
    pub default_name: String,
}

#[derive(Debug, Clone)]
pub struct BoardInfo {
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
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub read_only: bool,
}
