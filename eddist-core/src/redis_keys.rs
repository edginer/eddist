// Redis key helpers
pub fn csrf_key(key: &str) -> String {
    format!("csrf-token:{key}")
}

pub fn thread_cache_key(board_key: &str, thread_number: u64) -> String {
    format!("thread:{board_key}:{thread_number}")
}

pub fn res_creation_span_key(authed_token: &str) -> String {
    format!("res_creation_span:{authed_token}")
}

pub fn res_creation_span_ip_key(ip: &str) -> String {
    format!("res_creation_span_ip:{ip}")
}

pub fn thread_creation_span_key(authed_token: &str) -> String {
    format!("thread_creation_span:{authed_token}")
}

pub fn thread_creation_span_ip_key(ip: &str) -> String {
    format!("thread_creation_span_ip:{ip}")
}

pub fn res_creation_penalty_key(authed_token: &str) -> String {
    format!("res_creation_penalty:{authed_token}")
}

pub fn res_creation_long_restrict_key(authed_token: &str) -> String {
    format!("res_creation_long_restrict:{authed_token}")
}

pub fn user_session_key(user_sid: &str) -> String {
    format!("user:session:{user_sid}")
}

pub fn user_reg_temp_url_register_key(temp_url_query: &str) -> String {
    format!("userreg:tempurl:register:{temp_url_query}")
}

pub fn user_reg_oauth2_state_key(state_id: &str) -> String {
    format!("userreg:oauth2:state:{state_id}")
}

pub fn user_reg_oauth2_authreq_key(state_id: &str) -> String {
    format!("userreg:oauth2:authreq:{state_id}")
}

pub fn user_login_oauth2_authreq_key(state_id: &str) -> String {
    format!("userlogin:oauth2:authreq:{state_id}")
}

pub fn email_auth_used_key(token: &str) -> String {
    format!("resp:email_auth_used:{token}")
}

// Channel constants
pub const CHANNEL_RES_CREATED: &str = "bbs:event:res_created";
pub const CHANNEL_THREAD_CREATED: &str = "bbs:event:thread_created";
pub use crate::domain::pubsub_repository::{
    CHANNEL_AUTH_TOKEN_INITIATED, CHANNEL_AUTH_TOKEN_REQUESTED, CHANNEL_AUTH_TOKEN_REVOKED,
    CHANNEL_AUTH_TOKEN_SUCCEEDED, CHANNEL_PUBSUB_ITEM,
};
