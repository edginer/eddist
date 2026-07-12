use std::env;

use ::redis::AsyncCommands as _;
use base64::Engine;
use eddist_core::redis_keys::csrf_key;
use eddist_core::{domain::tinker::Tinker, utils::is_prod};
use http::HeaderMap;
use sqlx::{Database, Transaction};
use uuid::Uuid;

/// Returns the client origin IP, or `None` in production when no valid CDN-provided
/// header is present. In non-prod, a missing/invalid header falls back to `localhost`.
pub fn get_origin_ip(headers: &HeaderMap) -> Option<&str> {
    let origin_ip = headers
        .get("Cf-Connecting-IP")
        .or_else(|| headers.get("X-Forwarded-For"))
        .and_then(|x| x.to_str().ok());

    match origin_ip {
        Some(ip) => Some(ip),
        None if !is_prod() => Some("localhost"),
        None => None,
    }
}

/// Returns the request User-Agent, or `None` in production when it is
/// missing/invalid. In non-prod, a missing/invalid header falls back to `unknown`.
pub fn get_ua(headers: &HeaderMap) -> Option<&str> {
    match headers.get("User-Agent").and_then(|x| x.to_str().ok()) {
        Some(ua) => Some(ua),
        None if !is_prod() => Some("unknown"),
        None => None,
    }
}

/// Returns the client ASN, or `None` in production when no valid CDN-provided
/// header is present. In non-prod, a missing/invalid header falls back to `0`.
pub fn get_asn_num(headers: &HeaderMap) -> Option<u32> {
    let header_name = env::var("ASN_NUMBER_HEADER_NAME").unwrap_or("X-ASN-Num".to_string());

    let asn = headers
        .get(header_name)
        .and_then(|x| x.to_str().ok())
        .and_then(|x| x.parse::<u32>().ok());

    match asn {
        Some(asn) => Some(asn),
        None if !is_prod() => Some(0),
        None => None,
    }
}

pub fn get_tinker(tinker: &str, secret: &str) -> Option<Tinker> {
    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.validate_exp = false;
    validation.validate_nbf = false;
    validation.validate_aud = false;
    validation.required_spec_claims.clear();

    let secret_bytes = base64::engine::general_purpose::STANDARD
        .decode(secret.trim())
        .ok()?;

    let tinker = jsonwebtoken::decode::<Tinker>(
        tinker,
        &jsonwebtoken::DecodingKey::from_secret(&secret_bytes),
        &validation,
    )
    .ok()?
    .claims;

    // Legacy cookies pre-date the internal_level field; promote to level so restrictions are unchanged.
    let level = tinker.level();
    Some(tinker.patch_internal_level_if_missing(level))
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
        assert_eq!(get_origin_ip(&headers), Some("203.0.113.1"));
        unsafe { std::env::remove_var("ENV") };
    }

    #[test]
    fn test_get_origin_ip_x_forwarded() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Forwarded-For", "198.51.100.1".parse().unwrap());

        unsafe { std::env::set_var("ENV", "production") };
        assert_eq!(get_origin_ip(&headers), Some("198.51.100.1"));
        unsafe { std::env::remove_var("ENV") };
    }

    #[test]
    fn test_get_origin_ip_localhost_fallback() {
        let headers = HeaderMap::new();
        assert_eq!(get_origin_ip(&headers), Some("localhost"));
    }

    #[test]
    fn test_get_ua_present() {
        let mut headers = HeaderMap::new();
        headers.insert("User-Agent", "Mozilla/5.0 Test".parse().unwrap());
        assert_eq!(get_ua(&headers), Some("Mozilla/5.0 Test"));
    }

    #[test]
    fn test_get_ua_missing() {
        let headers = HeaderMap::new();
        assert_eq!(get_ua(&headers), Some("unknown"));
    }

    #[test]
    fn test_get_asn_num_missing_fallback() {
        let headers = HeaderMap::new();
        assert_eq!(get_asn_num(&headers), Some(0));
    }
}
