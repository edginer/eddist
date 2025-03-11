use std::env;

use base64::Engine;
use eddist_core::{domain::tinker::Tinker, utils::is_prod};
use http::HeaderMap;
use jwt_simple::prelude::MACLike;
use sqlx::{Database, Transaction};

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
    let key = jwt_simple::prelude::HS256Key::from_bytes(
        &base64::engine::general_purpose::STANDARD
            .decode(secret.trim())
            .unwrap(),
    );
    let tinker = key.verify_token::<Tinker>(tinker, None).ok()?;

    Some(tinker.custom)
}

#[async_trait::async_trait]
pub trait TransactionRepository<T: Database> {
    async fn begin(&self) -> anyhow::Result<Transaction<'_, T>>;
    async fn commit(&self, tx: Transaction<'_, T>) -> anyhow::Result<()> {
        tx.commit().await?;
        Ok(())
    }
    async fn rollback(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[macro_export]
macro_rules! transaction_repository {
    ($impl_struct:ident, $conn:ident, $database:ident) => {
        #[async_trait::async_trait]
        impl crate::utils::TransactionRepository<$database> for $impl_struct {
            async fn begin(&self) -> anyhow::Result<::sqlx::Transaction<'_, $database>> {
                let tx = self.$conn.begin().await?;
                Ok(tx)
            }
        }
    };
}
