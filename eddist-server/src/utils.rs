use std::env;

use ::redis::AsyncCommands as _;
use base64::Engine;
use eddist_core::{domain::tinker::Tinker, utils::is_prod};
use http::HeaderMap;
use redis::csrf_key;
use sqlx::{Database, Transaction};
use uuid::Uuid;

pub(crate) mod redis {
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

    pub fn user_link_onetime_key(key: &str) -> String {
        format!("userlink:onetime:{key}")
    }

    pub fn user_link_oauth2_state_key(state_id: &str) -> String {
        format!("userlink:oauth2:state:{state_id}")
    }

    pub fn user_link_oauth2_authreq_key(state_id: &str) -> String {
        format!("userlink:oauth2:authreq:{state_id}")
    }

    pub fn email_auth_used_key(token: &str) -> String {
        format!("resp:email_auth_used:{token}")
    }
}

pub fn get_origin_ip(headers: &HeaderMap) -> &str {
    let origin_ip = headers
        .get("Cf-Connecting-IP")
        .or_else(|| headers.get("X-Forwarded-For"))
        .map(|x| x.to_str());

    if is_prod() {
        origin_ip.unwrap().unwrap()
    } else {
        origin_ip.unwrap_or(Ok("localhost")).unwrap()
    }
}

pub fn get_ua(headers: &HeaderMap) -> &str {
    headers
        .get("User-Agent")
        .map(|x| x.to_str())
        .unwrap_or(Ok("unknown"))
        .unwrap()
}

pub fn get_asn_num(headers: &HeaderMap) -> u32 {
    let header_name = env::var("ASN_NUMBER_HEADER_NAME").unwrap_or("X-ASN-Num".to_string());

    let header = headers.get(header_name).map(|x| x.to_str());

    if is_prod() {
        header.unwrap().unwrap().parse::<u32>().unwrap()
    } else {
        header.unwrap_or(Ok("0")).unwrap().parse::<u32>().unwrap()
    }
}

pub fn get_tinker(tinker: &str, secret: &str) -> Option<Tinker> {
    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.validate_exp = false;
    validation.validate_nbf = false;
    validation.validate_aud = false;
    validation.required_spec_claims.clear();

    let tinker = jsonwebtoken::decode::<Tinker>(
        tinker,
        &jsonwebtoken::DecodingKey::from_secret(
            &base64::engine::general_purpose::STANDARD
                .decode(secret.trim())
                .unwrap(),
        ),
        &validation,
    )
    .unwrap();

    Some(tinker.claims)
}

#[async_trait::async_trait]
pub trait TransactionRepository<T: Database> {
    async fn begin(&self) -> anyhow::Result<Transaction<'_, T>>;
}

#[macro_export]
macro_rules! transaction_repository {
    ($impl_struct:ident, $conn:ident, $database:ident) => {
        #[async_trait::async_trait]
        impl $crate::utils::TransactionRepository<$database> for $impl_struct {
            async fn begin(&self) -> anyhow::Result<::sqlx::Transaction<'_, $database>> {
                let tx = self.$conn.begin().await?;
                Ok(tx)
            }
        }
    };
}

#[derive(Clone)]
pub struct CsrfState {
    redis: ::redis::aio::ConnectionManager,
}

impl CsrfState {
    pub fn new(redis: ::redis::aio::ConnectionManager) -> Self {
        Self { redis }
    }

    /// Generate a new CSRF token
    /// Key is must be HTTP header value (does not allow some special characters like `:`)
    pub async fn generate_new_csrf_token(&self, key: &str, ttl: u64) -> anyhow::Result<String> {
        let mut conn = self.redis.clone();

        let token = base64::engine::general_purpose::STANDARD.encode(
            [
                Uuid::now_v7().as_bytes().as_slice(),
                Uuid::now_v7().as_bytes(),
            ]
            .concat(),
        );
        let token = token.trim_end_matches('=');

        let csrf_token = format!("{key}-{token}");

        conn.set_ex::<_, _, ()>(csrf_key(&csrf_token), "", ttl)
            .await?;

        Ok(csrf_token)
    }

    pub async fn verify_csrf_token(&self, key_token: &str) -> anyhow::Result<bool> {
        let mut conn = self.redis.clone();

        let csrf_result = conn
            .get_del::<_, Option<String>>(&csrf_key(key_token))
            .await?;

        Ok(csrf_result.is_some())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    #[test]
    fn test_get_origin_ip_cloudflare() {
        let mut headers = HeaderMap::new();
        headers.insert("Cf-Connecting-IP", "203.0.113.1".parse().unwrap());

        std::env::set_var("ENV", "production");
        assert_eq!(get_origin_ip(&headers), "203.0.113.1");
        std::env::remove_var("ENV");
    }

    #[test]
    fn test_get_origin_ip_x_forwarded() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Forwarded-For", "198.51.100.1".parse().unwrap());

        std::env::set_var("ENV", "production");
        assert_eq!(get_origin_ip(&headers), "198.51.100.1");
        std::env::remove_var("ENV");
    }

    #[test]
    fn test_get_origin_ip_localhost_fallback() {
        let headers = HeaderMap::new();
        assert_eq!(get_origin_ip(&headers), "localhost");
    }

    #[test]
    fn test_get_ua_present() {
        let mut headers = HeaderMap::new();
        headers.insert("User-Agent", "Mozilla/5.0 Test".parse().unwrap());
        assert_eq!(get_ua(&headers), "Mozilla/5.0 Test");
    }

    #[test]
    fn test_get_ua_missing() {
        let headers = HeaderMap::new();
        assert_eq!(get_ua(&headers), "unknown");
    }
}
