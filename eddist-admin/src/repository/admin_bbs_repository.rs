use eddist_core::domain::client_info::ClientInfo;
use sqlx::types::Json;

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
    pub is_abone_keep_id: i8,
    pub res_order: i32,
    pub client_info: Json<ClientInfo>,
}
