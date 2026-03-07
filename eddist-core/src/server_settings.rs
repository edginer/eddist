pub const KEY_ENABLE_IDP_LINKING: &str = "user.enable_idp_linking";
pub const KEY_REQUIRE_IDP_LINKING: &str = "user.require_idp_linking";

pub enum ServerSettingKey {
    EnableIdpLinking,
    RequireIdpLinking,
}

impl ServerSettingKey {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::EnableIdpLinking => KEY_ENABLE_IDP_LINKING,
            Self::RequireIdpLinking => KEY_REQUIRE_IDP_LINKING,
        }
    }

    pub const ALL: &[ServerSettingKey] = &[
        ServerSettingKey::EnableIdpLinking,
        ServerSettingKey::RequireIdpLinking,
    ];

    pub const fn description(&self) -> &'static str {
        match self {
            Self::EnableIdpLinking => "Enable the IdP account linking feature (true/false)",
            Self::RequireIdpLinking => {
                "Require users to link an external IdP account before posting. Only applies to auth tokens issued after enabling this setting. (true/false)"
            }
        }
    }
}

impl std::fmt::Display for ServerSettingKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
