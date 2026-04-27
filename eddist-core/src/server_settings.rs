pub const KEY_ENABLE_IDP_LINKING: &str = "user.enable_idp_linking";
pub const KEY_REQUIRE_IDP_LINKING: &str = "user.require_idp_linking";
pub const KEY_AI_OPENAI_API_KEY: &str = "ai.openai_api_key";
pub const KEY_AI_MODERATION_ON_RES: &str = "ai.moderation_on_res";
pub const KEY_AI_MODERATION_ON_THREAD: &str = "ai.moderation_on_thread";

pub enum ServerSettingKey {
    EnableIdpLinking,
    RequireIdpLinking,
    AiOpenAiApiKey,
    AiModerationOnRes,
    AiModerationOnThread,
}

impl ServerSettingKey {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::EnableIdpLinking => KEY_ENABLE_IDP_LINKING,
            Self::RequireIdpLinking => KEY_REQUIRE_IDP_LINKING,
            Self::AiOpenAiApiKey => KEY_AI_OPENAI_API_KEY,
            Self::AiModerationOnRes => KEY_AI_MODERATION_ON_RES,
            Self::AiModerationOnThread => KEY_AI_MODERATION_ON_THREAD,
        }
    }

    pub const ALL: &[ServerSettingKey] = &[
        ServerSettingKey::EnableIdpLinking,
        ServerSettingKey::RequireIdpLinking,
        ServerSettingKey::AiOpenAiApiKey,
        ServerSettingKey::AiModerationOnRes,
        ServerSettingKey::AiModerationOnThread,
    ];

    pub const fn description(&self) -> &'static str {
        match self {
            Self::EnableIdpLinking => "Enable the IdP account linking feature (true/false)",
            Self::RequireIdpLinking => {
                "Require users to link an external IdP account before posting. Only applies to auth tokens issued after enabling this setting. (true/false)"
            }
            Self::AiOpenAiApiKey => {
                "OpenAI API key for content moderation (encrypted with TINKER_SECRET)"
            }
            Self::AiModerationOnRes => {
                "Enable OpenAI content moderation for responses (true/false)"
            }
            Self::AiModerationOnThread => {
                "Enable OpenAI content moderation for thread creation (true/false)"
            }
        }
    }
}

impl std::fmt::Display for ServerSettingKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
