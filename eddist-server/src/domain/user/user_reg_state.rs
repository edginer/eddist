use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserRegState {
    pub authed_token: String,
    pub edge_token: Option<String>,
    pub idp_name: Option<String>,
    pub nonce: Option<String>,
    pub code_verifier: Option<String>,
}
