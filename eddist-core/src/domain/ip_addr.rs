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

impl ReducedIpAddr {
    pub fn is_v4(&self) -> bool {
        matches!(self, Self::V4(_))
    }

    pub fn is_v6(&self) -> bool {
        matches!(self, Self::V6(_))
    }
}

impl From<IpAddr> for ReducedIpAddr {
    fn from(value: IpAddr) -> Self {
        value.0.into()
    }
}

impl From<String> for ReducedIpAddr {
    fn from(value: String) -> Self {
        if let Ok(v6) = value.parse::<std::net::Ipv6Addr>() {
            // Full (possibly compressed) v6 address: reduce to the /64 prefix.
            let segments = v6.segments();
            Self::V6([
                format!("{:x}", segments[0]),
                format!("{:x}", segments[1]),
                format!("{:x}", segments[2]),
                format!("{:x}", segments[3]),
            ])
        } else if value.contains(':') {
            // Already-reduced v6 string (e.g. a stored reduced_origin_ip).
            let mut parts = value.split(':').map(|s| s.to_string());
            Self::V6([
                parts.next().unwrap_or_default(),
                parts.next().unwrap_or_default(),
                parts.next().unwrap_or_default(),
                parts.next().unwrap_or_default(),
            ])
        } else {
            // v4
            Self::V4(value)
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
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reduced_ip_addr_v4() {
        let ip = ReducedIpAddr::from("192.168.1.1".to_string());
        assert!(ip.is_v4());
        assert!(!ip.is_v6());
        assert_eq!(ip.to_string(), "192.168.1.1");
    }

    #[test]
    fn test_reduced_ip_addr_v6() {
        let ip = ReducedIpAddr::from("2001:db8:85a3:0".to_string());
        assert!(ip.is_v6());
        assert!(!ip.is_v4());
        assert_eq!(ip.to_string(), "2001:db8:85a3:0");
    }

    #[test]
    fn test_reduced_ip_addr_v6_compressed() {
        let ip = ReducedIpAddr::from("::1".to_string());
        assert!(ip.is_v6());
        assert_eq!(ip.to_string(), "0:0:0:0");

        let ip = ReducedIpAddr::from("fe80::1".to_string());
        assert!(ip.is_v6());
        assert_eq!(ip.to_string(), "fe80:0:0:0");

        let ip = ReducedIpAddr::from("2001:db8:85a3:0:0:8a2e:370:7334".to_string());
        assert!(ip.is_v6());
        assert_eq!(ip.to_string(), "2001:db8:85a3:0");
    }

    #[test]
    fn test_ip_addr_conversion() {
        let ip = IpAddr::new("10.0.0.1".to_string());
        let reduced: ReducedIpAddr = ip.into();
        assert!(reduced.is_v4());
        assert_eq!(reduced.to_string(), "10.0.0.1");
    }
}
