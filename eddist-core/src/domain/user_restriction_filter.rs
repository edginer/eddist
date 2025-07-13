use serde::{Deserialize, Serialize};
use wirefilter::{ExecutionContext, Scheme};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserAttributeFilter {
    pub expression: String,
}

#[derive(Debug, Clone)]
pub struct UserAttributes {
    pub ip_addr: String,
    pub user_agent: String,
    pub asn_num: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum FilterParseError {
    #[error("Invalid syntax: {0}")]
    InvalidSyntax(String),
    #[error("Parse error: {0}")]
    ParseError(String),
}

impl UserAttributeFilter {
    pub fn new(expression: String) -> Result<Self, FilterParseError> {
        // Validate the expression by parsing it
        let scheme = Self::create_scheme();
        let _ast = scheme.parse(&expression)
            .map_err(|e| FilterParseError::ParseError(e.to_string()))?;
        
        Ok(Self { expression })
    }

    pub fn matches(&self, attributes: &UserAttributes) -> Result<bool, FilterParseError> {
        let scheme = Self::create_scheme();
        let ast = scheme.parse(&self.expression)
            .map_err(|e| FilterParseError::ParseError(e.to_string()))?;
        let filter = ast.compile();

        let mut ctx = ExecutionContext::new(&scheme);
        
        // Add user attributes to execution context
        let ip_field = scheme.get_field("ip").map_err(|e| FilterParseError::InvalidSyntax(e.to_string()))?;
        let ua_field = scheme.get_field("ua").map_err(|e| FilterParseError::InvalidSyntax(e.to_string()))?;
        let asn_field = scheme.get_field("asn").map_err(|e| FilterParseError::InvalidSyntax(e.to_string()))?;
        
        ctx.set_field_value(ip_field, attributes.ip_addr.clone())
            .map_err(|e| FilterParseError::InvalidSyntax(e.to_string()))?;
        ctx.set_field_value(ua_field, attributes.user_agent.clone())
            .map_err(|e| FilterParseError::InvalidSyntax(e.to_string()))?;
        ctx.set_field_value(asn_field, attributes.asn_num as i32)
            .map_err(|e| FilterParseError::InvalidSyntax(e.to_string()))?;

        let result = filter.execute(&ctx)
            .map_err(|e| FilterParseError::InvalidSyntax(e.to_string()))?;

        Ok(result)
    }

    fn create_scheme() -> Scheme {
        let scheme = Scheme! {
            // IP address field
            ip: Ip,
            
            // User agent field  
            ua: Bytes,
            
            // ASN field
            asn: Int,
        };
        scheme.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_attributes() -> UserAttributes {
        UserAttributes {
            ip_addr: "192.168.1.100".to_string(),
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/91.0".to_string(),
            asn_num: 12345,
        }
    }

    #[test]
    fn test_ip_equals() {
        let filter = UserAttributeFilter::new("ip == 192.168.1.100".to_string()).unwrap();
        assert!(filter.matches(&test_attributes()).unwrap());

        let filter = UserAttributeFilter::new("ip == 192.168.1.101".to_string()).unwrap();
        assert!(!filter.matches(&test_attributes()).unwrap());
    }

    #[test]
    fn test_ua_contains() {
        let filter = UserAttributeFilter::new(r#"ua contains "Chrome""#.to_string()).unwrap();
        assert!(filter.matches(&test_attributes()).unwrap());

        let filter = UserAttributeFilter::new(r#"ua contains "Firefox""#.to_string()).unwrap();
        assert!(!filter.matches(&test_attributes()).unwrap());
    }

    #[test]
    fn test_asn_equals() {
        let filter = UserAttributeFilter::new("asn == 12345".to_string()).unwrap();
        assert!(filter.matches(&test_attributes()).unwrap());

        let filter = UserAttributeFilter::new("asn == 54321".to_string()).unwrap();
        assert!(!filter.matches(&test_attributes()).unwrap());
    }

    #[test]
    fn test_logical_and() {
        let filter = UserAttributeFilter::new(r#"ip == 192.168.1.100 and ua contains "Chrome""#.to_string()).unwrap();
        assert!(filter.matches(&test_attributes()).unwrap());

        let filter = UserAttributeFilter::new(r#"ip == 192.168.1.100 and ua contains "Firefox""#.to_string()).unwrap();
        assert!(!filter.matches(&test_attributes()).unwrap());
    }

    #[test]
    fn test_logical_or() {
        let filter = UserAttributeFilter::new(r#"ip == 192.168.1.101 or ua contains "Chrome""#.to_string()).unwrap();
        assert!(filter.matches(&test_attributes()).unwrap());

        let filter = UserAttributeFilter::new(r#"ip == 192.168.1.101 or ua contains "Firefox""#.to_string()).unwrap();
        assert!(!filter.matches(&test_attributes()).unwrap());
    }

    #[test]
    fn test_not_operator() {
        let filter = UserAttributeFilter::new(r#"not ua contains "Firefox""#.to_string()).unwrap();
        assert!(filter.matches(&test_attributes()).unwrap());

        let filter = UserAttributeFilter::new(r#"not ua contains "Chrome""#.to_string()).unwrap();
        assert!(!filter.matches(&test_attributes()).unwrap());
    }

    #[test]
    fn test_ip_in_range() {
        let filter = UserAttributeFilter::new("ip in 192.168.1.0/24".to_string()).unwrap();
        assert!(filter.matches(&test_attributes()).unwrap());

        let filter = UserAttributeFilter::new("ip in 10.0.0.0/8".to_string()).unwrap();
        assert!(!filter.matches(&test_attributes()).unwrap());
    }

    #[test]
    fn test_complex_expression() {
        let filter = UserAttributeFilter::new(
            r#"(ip in 192.168.0.0/16 or asn == 12345) and ua contains "Chrome""#.to_string()
        ).unwrap();
        assert!(filter.matches(&test_attributes()).unwrap());

        let filter = UserAttributeFilter::new(
            r#"(ip in 10.0.0.0/8 and asn == 54321) or ua contains "Firefox""#.to_string()
        ).unwrap();
        assert!(!filter.matches(&test_attributes()).unwrap());
    }
}