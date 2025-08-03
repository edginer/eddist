use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct Board {
    pub id: Uuid,
    pub name: String,
    pub board_key: String,
    pub default_name: String,
    pub thread_count: i64,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct BoardInfo {
    pub local_rules: String,
    pub base_thread_creation_span_sec: usize,
    pub base_response_creation_span_sec: usize,
    pub max_thread_name_byte_length: usize,
    pub max_author_name_byte_length: usize,
    pub max_email_byte_length: usize,
    pub max_response_body_byte_length: usize,
    pub max_response_body_lines: usize,
    pub threads_archive_cron: Option<String>,
    pub threads_archive_trigger_thread_count: Option<usize>,
    pub read_only: bool,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct CreateBoardInput {
    pub name: String,
    pub board_key: String,
    pub default_name: String,
    pub local_rule: String,
    pub base_thread_creation_span_sec: Option<usize>,
    pub base_response_creation_span_sec: Option<usize>,
    pub max_thread_name_byte_length: Option<usize>,
    pub max_author_name_byte_length: Option<usize>,
    pub max_email_byte_length: Option<usize>,
    pub max_response_body_byte_length: Option<usize>,
    pub max_response_body_lines: Option<usize>,
    pub threads_archive_cron: Option<String>,
    pub threads_archive_trigger_thread_count: Option<usize>,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct EditBoardInput {
    pub name: Option<String>,
    pub default_name: Option<String>,
    pub local_rule: Option<String>,
    pub base_thread_creation_span_sec: Option<usize>,
    pub base_response_creation_span_sec: Option<usize>,
    pub max_thread_name_byte_length: Option<usize>,
    pub max_author_name_byte_length: Option<usize>,
    pub max_email_byte_length: Option<usize>,
    pub max_response_body_byte_length: Option<usize>,
    pub max_response_body_lines: Option<usize>,
    pub threads_archive_cron: Option<String>,
    pub threads_archive_trigger_thread_count: Option<usize>,
    pub read_only: Option<bool>,
}
