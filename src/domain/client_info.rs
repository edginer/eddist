use super::{ip_addr::IpAddr, tinker::Tinker};

#[derive(Debug, Clone)]
pub struct ClientInfo {
    pub user_agent: String,
    pub asn_num: u32,
    pub ip_addr: String,
    pub tinker: Option<Box<Tinker>>,
}

impl ClientInfo {
    pub fn ip_addr(&self) -> IpAddr {
        IpAddr::new(self.ip_addr.clone())
    }
}
