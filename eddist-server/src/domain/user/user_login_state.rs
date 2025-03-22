use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserLoginState {
    pub idp_name: String,
    pub nonce: String,
    pub code_verifier: String,
    pub user_login_state_id: String,
}
