use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RegistrationSource {
    BbsCgi,
    AuthCode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TempUrlRegistrationRecord {
    pub authed_token_id: String,
    pub source: RegistrationSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRegState {
    pub authed_token: String,
    pub edge_token: Option<String>,
    pub idp_name: Option<String>,
    pub nonce: Option<String>,
    pub code_verifier: Option<String>,
    pub source: RegistrationSource,
}
