use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Idp {
    pub id: Uuid,
    pub idp_name: String,
    pub idp_display_name: String,
    pub idp_logo_svg: Option<String>,
    pub oidc_config_url: String,
    pub client_id: String,
    pub client_secret: String, // encrypted
    pub enabled: bool,
}
