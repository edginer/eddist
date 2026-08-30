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

pub fn authed_token_suspended_key(authed_token_id: &str) -> String {
    format!("authed_token:suspended:{authed_token_id}")
}

pub fn tripwire_uuid_seen_key(uuid: &str) -> String {
    format!("captcha:tripwire:uuid:{uuid}")
}

pub fn reauth_temp_key(temp_key: &str) -> String {
    format!("reauth:temp:{temp_key}")
}

pub fn reauth_lock_key(token_id: &str) -> String {
    format!("reauth:lock:{token_id}")
}

pub fn unsafe_threads_key(board_id: impl std::fmt::Display) -> String {
    format!("bbs:safe_mode:unsafe_threads:{board_id}")
}

pub fn not_found_access_count_key(ip: &str) -> String {
    format!("not_found:count:{ip}")
}

pub fn shared_ng_id_key(board_key: &str, ng_id: &str) -> String {
    format!("shared_ng_id:{board_key}:{ng_id}")
}

pub fn shared_ng_thread_metadent_key(board_key: &str, metadent: &str) -> String {
    format!("shared_ng_thread_metadent:{board_key}:{metadent}")
}

pub fn shared_ng_id_active_key(board_key: &str) -> String {
    format!("shared_ng_id_active:{board_key}")
}

pub fn shared_ng_thread_metadent_active_key(board_key: &str) -> String {
    format!("shared_ng_thread_metadent_active:{board_key}")
}

pub fn shared_ng_id_rate_limit_key(token_hash: &str) -> String {
    format!("shared_ng_id:rate_limit:{token_hash}")
}

pub fn shared_ng_id_ip_rate_limit_key(reduced_ip: &str) -> String {
    format!("shared_ng_id:ip_rate_limit:{reduced_ip}")
}

pub const DB_FAILED_CACHE_RES_KEY: &str = "bbs:db_failed_cache:res";

pub const CHANNEL_RES_CREATED: &str = "bbs:event:res_created";
pub const CHANNEL_THREAD_CREATED: &str = "bbs:event:thread_created";
pub use crate::domain::pubsub_repository::{
    CHANNEL_AUTH_TOKEN_INITIATED, CHANNEL_AUTH_TOKEN_REQUESTED, CHANNEL_AUTH_TOKEN_REVOKED,
    CHANNEL_AUTH_TOKEN_SUCCEEDED, CHANNEL_PUBSUB_ITEM,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shared_ng_id_ip_rate_limit_key_v4() {
        assert_eq!(
            shared_ng_id_ip_rate_limit_key("203.0.113.1"),
            "shared_ng_id:ip_rate_limit:203.0.113.1"
        );
    }

    #[test]
    fn test_shared_ng_id_ip_rate_limit_key_reduced_v6() {
        assert_eq!(
            shared_ng_id_ip_rate_limit_key("2001:db8:85a3:0"),
            "shared_ng_id:ip_rate_limit:2001:db8:85a3:0"
        );
    }

    #[test]
    fn test_active_keys_do_not_collide_with_contributor_sets() {
        assert_eq!(shared_ng_id_active_key("news"), "shared_ng_id_active:news");
        assert_ne!(
            shared_ng_id_active_key("news"),
            shared_ng_id_key("active", "news")
        );
        assert_eq!(
            shared_ng_thread_metadent_active_key("news"),
            "shared_ng_thread_metadent_active:news"
        );
    }

    #[test]
    fn test_shared_ng_thread_metadent_key() {
        assert_eq!(
            shared_ng_thread_metadent_key("news", "abcd1234"),
            "shared_ng_thread_metadent:news:abcd1234"
        );
    }
}
