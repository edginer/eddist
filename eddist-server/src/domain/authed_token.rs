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
    pub writing_ua: String,
    pub authed_ua: Option<String>,
    pub auth_code: String,
    pub created_at: DateTime<Utc>,
    pub authed_at: Option<DateTime<Utc>>,
    pub validity: bool,
    pub last_wrote_at: Option<DateTime<Utc>>,
}

impl AuthedToken {
    pub fn new(origin_ip: String, writing_ua: String) -> Self {
        let id = Uuid::now_v7();
        let token = md5::Md5::new()
            .chain_update(id.as_bytes())
            .chain_update(origin_ip.as_bytes())
            .chain_update(writing_ua.as_bytes())
            .finalize();
        let token = format!("{token:x}");
        let ip_addr = IpAddr::new(origin_ip);
        let reduced_ip = ip_addr.clone().into();
        let auth_code = rand::thread_rng().gen_range(0..1000000);
        let auth_code = format!("{auth_code:06}");

        Self {
            id,
            token,
            origin_ip: ip_addr,
            reduced_ip,
            writing_ua,
            authed_ua: None,
            auth_code,
            created_at: Utc::now(),
            authed_at: None,
            validity: false,
            last_wrote_at: None,
        }
    }

    pub fn is_activation_expired(&self, now: DateTime<Utc>) -> bool {
        self.created_at.timestamp() + 60 * 5 < now.timestamp()
    }
}
