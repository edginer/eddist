use chrono::{DateTime, Utc};
use eddist_core::{
    domain::ip_addr::{IpAddr, ReducedIpAddr},
    utils::to_hex,
};
use md5::{self, Digest};
use rand::RngExt;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AuthedToken {
    pub id: Uuid,
    pub token: String,
    pub origin_ip: IpAddr,
    pub reduced_ip: ReducedIpAddr,
    pub asn_num: i32,
    pub writing_ua: String,
    pub authed_ua: Option<String>,
    pub auth_code: String,
    pub created_at: DateTime<Utc>,
    pub authed_at: Option<DateTime<Utc>>,
    pub validity: bool,
    pub last_wrote_at: Option<DateTime<Utc>>,
    pub author_id_seed: Vec<u8>,
    pub require_user_registration: bool,
    pub registered_user_id: Option<Uuid>,
    pub require_reauth: bool,
}

impl AuthedToken {
    pub fn new(origin_ip: String, writing_ua: String, asn_num: i32) -> Self {
        let id = Uuid::now_v7();
        let token = md5::Md5::new()
            .chain_update(id.as_bytes())
            .chain_update(origin_ip.as_bytes())
            .chain_update(writing_ua.as_bytes())
            .finalize();
        let token = to_hex(token);
        let ip_addr = IpAddr::new(origin_ip);
        let reduced_ip = ReducedIpAddr::from(ip_addr.clone());
        let auth_code = rand::rng().random_range(0..1000000);
        let auth_code = format!("{auth_code:06}");

        Self {
            id,
            token,
            origin_ip: ip_addr,
            reduced_ip: reduced_ip.clone(),
            asn_num,
            writing_ua,
            authed_ua: None,
            auth_code,
            created_at: Utc::now(),
            authed_at: None,
            validity: false,
            last_wrote_at: None,
            author_id_seed: sha2::Sha512::digest(reduced_ip.to_string().as_bytes()).to_vec(),
            require_user_registration: false,
            registered_user_id: None,
            require_reauth: false,
        }
    }

    pub fn is_activation_expired(&self, now: DateTime<Utc>) -> bool {
        self.created_at.timestamp() + 60 * 5 < now.timestamp()
    }
}

#[cfg(test)]
mod probe_tests {
    use super::*;

    #[test]
    fn known_vectors_stable_across_digest_crate_bump() {
        let id = Uuid::from_bytes([1u8; 16]);
        let token = md5::Md5::new()
            .chain_update(id.as_bytes())
            .chain_update(b"1.2.3.4")
            .chain_update(b"test-ua")
            .finalize();
        assert_eq!(to_hex(token), "9b92d63778c2cd1ac5c4bf844cffeddf");

        assert_eq!(
            sha2::Sha512::digest(b"1.2.3.0").to_vec(),
            vec![
                194, 230, 125, 190, 87, 55, 12, 120, 241, 49, 15, 52, 29, 112, 97, 92, 50, 209,
                180, 59, 52, 84, 236, 190, 81, 217, 143, 73, 11, 74, 213, 57, 63, 131, 123, 11, 20,
                108, 143, 6, 67, 34, 28, 51, 231, 201, 115, 152, 79, 234, 18, 61, 126, 200, 10, 30,
                179, 138, 117, 39, 181, 235, 102, 207
            ]
        );
    }
}
