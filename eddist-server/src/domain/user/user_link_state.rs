use serde::{Deserialize, Serialize};

/// State struct for the user account linking flow.
/// Used when a user wants to link their authed_token to an external IdP account
/// after successfully completing the /auth-code authentication.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserLinkState {
    /// The token string of the authed_token to link
    pub authed_token: String,
    /// The authed_token ID (UUID) to bind to the user
    pub authed_token_id: String,
    /// The IdP name selected by the user
    pub idp_name: Option<String>,
    /// OIDC nonce for token validation
    pub nonce: Option<String>,
    /// PKCE code verifier
    pub code_verifier: Option<String>,
}
