use chrono::{DateTime, Utc};
use eddist_core::domain::ip_addr::{IpAddr, ReducedIpAddr};
use md5::{self, Digest};
use rand::Rng;
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
    pub registered_user_id: Option<Uuid>,
}

impl AuthedToken {
    pub fn new(origin_ip: String, writing_ua: String, asn_num: i32) -> Self {
        let id = Uuid::now_v7();
        let token = md5::Md5::new()
            .chain_update(id.as_bytes())
            .chain_update(origin_ip.as_bytes())
            .chain_update(writing_ua.as_bytes())
            .finalize();
        let token = format!("{token:x}");
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
            registered_user_id: None,
        }
    }

    pub fn is_activation_expired(&self, now: DateTime<Utc>) -> bool {
        self.created_at.timestamp() + 60 * 5 < now.timestamp()
    }
}
