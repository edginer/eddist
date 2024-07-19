use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpAddr(String);

impl IpAddr {
    pub fn new(ip: String) -> Self {
        Self(ip)
    }
}

impl Display for IpAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReducedIpAddr {
    V4(String),
    V6([String; 4]),
}

impl From<IpAddr> for ReducedIpAddr {
    fn from(value: IpAddr) -> Self {
        value.0.into()
    }
}

impl From<String> for ReducedIpAddr {
    fn from(value: String) -> Self {
        if value.contains(':') {
            // v6
            let split = value.split(':').collect::<Vec<_>>();
            Self::V6([
                split[0].to_string(),
                split[1].to_string(),
                split[2].to_string(),
                split[3].to_string(),
            ])
        } else {
            // v4
            Self::V4(value.to_string())
        }
    }
}

impl Display for ReducedIpAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReducedIpAddr::V4(s) => write!(f, "{s}"),
            ReducedIpAddr::V6(s) => write!(f, "{}:{}:{}:{}", s[0], s[1], s[2], s[3]),
        }
    }
}
