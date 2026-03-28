use std::env;

use ::redis::AsyncCommands as _;
use base64::Engine;
use eddist_core::redis_keys::csrf_key;
use eddist_core::{domain::tinker::Tinker, utils::is_prod};
use http::HeaderMap;
use sqlx::{Database, Transaction};
use uuid::Uuid;

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

        unsafe { std::env::set_var("ENV", "production") };
        assert_eq!(get_origin_ip(&headers), "203.0.113.1");
        unsafe { std::env::remove_var("ENV") };
    }

    #[test]
    fn test_get_origin_ip_x_forwarded() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Forwarded-For", "198.51.100.1".parse().unwrap());

        unsafe { std::env::set_var("ENV", "production") };
        assert_eq!(get_origin_ip(&headers), "198.51.100.1");
        unsafe { std::env::remove_var("ENV") };
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
