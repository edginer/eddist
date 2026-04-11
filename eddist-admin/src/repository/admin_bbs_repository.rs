use chrono::Utc;
use eddist_core::domain::client_info::ClientInfo;
#[cfg(feature = "backend-postgres")]
use sqlx::types::Json;
#[cfg(not(feature = "backend-postgres"))]
use sqlx::{FromRow, types::Json};
#[cfg(feature = "backend-postgres")]
use uuid::Uuid;

// Shared selection types used by admin_board_repository, admin_thread_repository,
// and admin_response_repository.

#[cfg(not(feature = "backend-postgres"))]
#[derive(Debug, FromRow)]
pub struct SelectionBoardWithThreadCount {
    pub id: Vec<u8>,
    pub name: String,
    pub board_key: String,
    pub default_name: String,
    pub thread_count: i64,
}

#[derive(Debug, sqlx::FromRow)]
pub struct SelectionBoardInfo {
    pub local_rules: String,
    pub base_thread_creation_span_sec: i32,
    pub base_response_creation_span_sec: i32,
    pub max_thread_name_byte_length: i32,
    pub max_author_name_byte_length: i32,
    pub max_email_byte_length: i32,
    pub max_response_body_byte_length: i32,
    pub max_response_body_lines: i32,
    pub threads_archive_trigger_thread_count: Option<i32>,
    pub threads_archive_cron: Option<String>,
    pub read_only: bool,
    pub force_metadent_type: Option<String>,
}

#[cfg(not(feature = "backend-postgres"))]
#[derive(Debug, FromRow)]
pub struct SelectionThread {
    pub id: Vec<u8>,
    pub board_id: Vec<u8>,
    pub thread_number: i64,
    pub last_modified_at: chrono::DateTime<Utc>,
    pub sage_last_modified_at: chrono::DateTime<Utc>,
    pub title: String,
    pub authed_token_id: Vec<u8>,
    pub metadent: String,
    pub response_count: i32,
    pub no_pool: bool,
    pub archived: bool,
    pub active: bool,
}

#[cfg(not(feature = "backend-postgres"))]
#[derive(Debug)]
pub struct SelectionRes {
    pub id: Vec<u8>,
    pub author_name: String,
    pub mail: String,
    pub body: String,
    pub created_at: chrono::NaiveDateTime,
    pub author_id: String,
    pub ip_addr: String,
    pub authed_token_id: Vec<u8>,
    pub board_id: Vec<u8>,
    pub thread_id: Vec<u8>,
    pub is_abone: i8,
    pub res_order: i32,
    pub client_info: Json<ClientInfo>,
}

// PostgreSQL-specific selection structs (native UUID, TIMESTAMPTZ → DateTime<Utc>, bool)
#[cfg(feature = "backend-postgres")]
#[derive(Debug, sqlx::FromRow)]
pub struct SelectionBoardWithThreadCountPg {
    pub id: Uuid,
    pub name: String,
    pub board_key: String,
    pub default_name: String,
    pub thread_count: i64,
}

#[cfg(feature = "backend-postgres")]
#[derive(Debug, sqlx::FromRow)]
pub struct SelectionThreadPg {
    pub id: Uuid,
    pub board_id: Uuid,
    pub thread_number: i64,
    pub last_modified_at: chrono::DateTime<Utc>,
    pub sage_last_modified_at: chrono::DateTime<Utc>,
    pub title: String,
    pub authed_token_id: Uuid,
    pub metadent: String,
    pub response_count: i32,
    pub no_pool: bool,
    pub archived: bool,
    pub active: bool,
}

#[cfg(feature = "backend-postgres")]
#[derive(Debug, sqlx::FromRow)]
pub struct SelectionResPg {
    pub id: Uuid,
    pub author_name: String,
    pub mail: String,
    pub body: String,
    pub created_at: chrono::DateTime<Utc>,
    pub author_id: String,
    pub ip_addr: String,
    pub authed_token_id: Uuid,
    pub board_id: Uuid,
    pub thread_id: Uuid,
    pub is_abone: bool,
    pub res_order: i32,
    pub client_info: Json<ClientInfo>,
}
