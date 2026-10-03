use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::net::IpAddr;
use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RestrictionRuleType {
    Asn,
    IP,
    IPCidr,
    UserAgent,
}

impl RestrictionRuleType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RestrictionRuleType::Asn => "ASN",
            RestrictionRuleType::IP => "IP",
            RestrictionRuleType::IPCidr => "IP_CIDR",
            RestrictionRuleType::UserAgent => "USER_AGENT",
        }
    }
}

impl FromStr for RestrictionRuleType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ASN" => Ok(RestrictionRuleType::Asn),
            "IP" => Ok(RestrictionRuleType::IP),
            "IP_CIDR" => Ok(RestrictionRuleType::IPCidr),
            "USER_AGENT" => Ok(RestrictionRuleType::UserAgent),
            _ => Err(format!("Invalid restriction rule type: {}", s)),
        }
    }
}

impl Display for RestrictionRuleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// The operation a rule blocks. Existing rules apply to both operations.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum RestrictionTarget {
    Authentication,
    Posting,
    #[default]
    Both,
}

impl RestrictionTarget {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Authentication => "AUTHENTICATION",
            Self::Posting => "POSTING",
            Self::Both => "BOTH",
        }
    }

    pub fn applies_to(self, operation: Self) -> bool {
        self == Self::Both || self == operation
    }
}

impl FromStr for RestrictionTarget {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "AUTHENTICATION" => Ok(Self::Authentication),
            "POSTING" => Ok(Self::Posting),
            "BOTH" => Ok(Self::Both),
            _ => Err(format!("Invalid restriction target: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRestrictionRule {
    pub id: Uuid,
    pub name: String,
    pub rule_type: RestrictionRuleType,
    #[serde(default)]
    pub target: RestrictionTarget,
    pub rule_value: String,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub created_by_email: String,
}

impl UserRestrictionRule {
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            chrono::Utc::now() > expires_at
        } else {
            false
        }
    }

    pub fn matches(&self, ip: &str, asn: u32, user_agent: &str) -> bool {
        if self.is_expired() {
            return false;
        }

        match self.rule_type {
            RestrictionRuleType::Asn => {
                if let Ok(rule_asn) = self.rule_value.parse::<u32>() {
                    rule_asn == asn
                } else {
                    false
                }
            }
            RestrictionRuleType::IP => self.rule_value == ip,
            RestrictionRuleType::IPCidr => {
                if let Ok(ip_addr) = ip.parse::<IpAddr>() {
                    if let Ok(cidr) = self.rule_value.parse::<ipnet::IpNet>() {
                        cidr.contains(&ip_addr)
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            RestrictionRuleType::UserAgent => {
                self.rule_value == user_agent || user_agent.contains(&self.rule_value)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRestrictionRuleInput {
    pub name: String,
    pub rule_type: RestrictionRuleType,
    #[serde(default)]
    pub target: RestrictionTarget,
    pub rule_value: String,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_by_email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserRestrictionRuleInput {
    pub id: Uuid,
    pub name: Option<String>,
    pub rule_type: Option<RestrictionRuleType>,
    pub target: Option<RestrictionTarget>,
    pub rule_value: Option<String>,
    pub expires_at: Option<Option<chrono::DateTime<chrono::Utc>>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restriction_targets_are_independent() {
        for (target, authentication, posting) in [
            (RestrictionTarget::Authentication, true, false),
            (RestrictionTarget::Posting, false, true),
            (RestrictionTarget::Both, true, true),
        ] {
            assert_eq!(
                target.applies_to(RestrictionTarget::Authentication),
                authentication
            );
            assert_eq!(target.applies_to(RestrictionTarget::Posting), posting);
            assert_eq!(
                target.as_str().parse::<RestrictionTarget>().unwrap(),
                target
            );
        }
    }

    #[test]
    fn legacy_rule_input_defaults_to_both() {
        let input: CreateUserRestrictionRuleInput = serde_json::from_value(serde_json::json!({
            "name": "legacy", "rule_type": "IP", "rule_value": "192.0.2.1",
            "expires_at": null, "created_by_email": "admin@example.com"
        }))
        .unwrap();
        assert_eq!(input.target, RestrictionTarget::Both);
    }
}
