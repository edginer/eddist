use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MetadentType {
    None,
    Verbose,
    VVerbose,
    VVVerbose,
}

impl From<MetadentType> for Option<&str> {
    fn from(value: MetadentType) -> Self {
        match value {
            MetadentType::None => None,
            MetadentType::Verbose => Some("v"),
            MetadentType::VVerbose => Some("vv"),
            MetadentType::VVVerbose => Some("vvv"),
        }
    }
}

impl From<&str> for MetadentType {
    fn from(value: &str) -> Self {
        match value {
            "v" => MetadentType::Verbose,
            "vv" => MetadentType::VVerbose,
            "vvv" => MetadentType::VVVerbose,
            _ => MetadentType::None,
        }
    }
}

impl std::str::FromStr for MetadentType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "v" => Ok(MetadentType::Verbose),
            "vv" => Ok(MetadentType::VVerbose),
            "vvv" => Ok(MetadentType::VVVerbose),
            _ => Err(()),
        }
    }
}
