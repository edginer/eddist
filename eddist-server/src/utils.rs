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
                Uuid::new_v4().as_bytes().as_slice(),
                Uuid::new_v4().as_bytes(),
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
