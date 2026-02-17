use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// IdP model for API responses (client_secret is never exposed)
#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct Idp {
    pub id: Uuid,
    pub idp_name: String,
    pub idp_display_name: String,
    pub idp_logo_svg: Option<String>,
    pub oidc_config_url: String,
    pub client_id: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct CreateIdpInput {
    pub idp_name: String,
    pub idp_display_name: String,
    pub idp_logo_svg: Option<String>,
    pub oidc_config_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, ToSchema, Serialize, Deserialize)]
pub struct UpdateIdpInput {
    pub idp_display_name: Option<String>,
    pub idp_logo_svg: Option<String>,
    pub oidc_config_url: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub enabled: Option<bool>,
}
