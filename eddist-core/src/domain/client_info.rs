use serde::{Deserialize, Serialize};

use super::{ip_addr::IpAddr, tinker::Tinker};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub user_agent: String,
    pub asn_num: u32,
    pub ip_addr: String,
    pub tinker: Option<Box<Tinker>>,
}

impl Default for ClientInfo {
    fn default() -> Self {
        Self {
            user_agent: String::new(),
            asn_num: 0,
            ip_addr: String::new(),
            tinker: None,
        }
    }
}

impl ClientInfo {
    pub fn ip_addr(&self) -> IpAddr {
        IpAddr::new(self.ip_addr.clone())
    }
}
