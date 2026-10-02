pub const KEY_CLOSE_NEW_AUTHENTICATION: &str = "bbs.close_new_authentication";
pub const KEY_ENABLE_IDP_LINKING: &str = "user.enable_idp_linking";
pub const KEY_REQUIRE_IDP_LINKING: &str = "user.require_idp_linking";
pub const KEY_AI_OPENAI_API_KEY: &str = "ai.openai_api_key";
pub const KEY_AI_MODERATION_ON_RES: &str = "ai.moderation_on_res";
pub const KEY_AI_MODERATION_ON_THREAD: &str = "ai.moderation_on_thread";
pub const KEY_ENABLE_SAFE_MODE: &str = "bbs.enable_safe_mode";
pub const KEY_AI_LLM_MODERATION_ON_THREAD: &str = "ai.llm_moderation_on_thread";
pub const KEY_AI_LLM_MODERATION_UNSAFE_THREADS: &str = "ai.llm_moderation_unsafe_threads";
pub const KEY_AI_LLM_MODERATION_MODEL: &str = "ai.llm_moderation_model";
pub const KEY_AI_LLM_MODERATION_INSTRUCTIONS: &str = "ai.llm_moderation_instructions";
pub const KEY_AI_LLM_MODERATION_INTERVAL_SECONDS: &str = "ai.llm_moderation_interval_seconds";

pub enum ServerSettingKey {
    CloseNewAuthentication,
    EnableIdpLinking,
    RequireIdpLinking,
    AiOpenAiApiKey,
    AiModerationOnRes,
    AiModerationOnThread,
    EnableSafeMode,
    AiLlmModerationOnThread,
    AiLlmModerationUnsafeThreads,
    AiLlmModerationModel,
    AiLlmModerationInstructions,
    AiLlmModerationIntervalSeconds,
}

impl ServerSettingKey {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::CloseNewAuthentication => KEY_CLOSE_NEW_AUTHENTICATION,
            Self::EnableIdpLinking => KEY_ENABLE_IDP_LINKING,
            Self::RequireIdpLinking => KEY_REQUIRE_IDP_LINKING,
            Self::AiOpenAiApiKey => KEY_AI_OPENAI_API_KEY,
            Self::AiModerationOnRes => KEY_AI_MODERATION_ON_RES,
            Self::AiModerationOnThread => KEY_AI_MODERATION_ON_THREAD,
            Self::EnableSafeMode => KEY_ENABLE_SAFE_MODE,
            Self::AiLlmModerationOnThread => KEY_AI_LLM_MODERATION_ON_THREAD,
            Self::AiLlmModerationUnsafeThreads => KEY_AI_LLM_MODERATION_UNSAFE_THREADS,
            Self::AiLlmModerationModel => KEY_AI_LLM_MODERATION_MODEL,
            Self::AiLlmModerationInstructions => KEY_AI_LLM_MODERATION_INSTRUCTIONS,
            Self::AiLlmModerationIntervalSeconds => KEY_AI_LLM_MODERATION_INTERVAL_SECONDS,
        }
    }

    pub const ALL: &[ServerSettingKey] = &[
        ServerSettingKey::CloseNewAuthentication,
        ServerSettingKey::EnableIdpLinking,
        ServerSettingKey::RequireIdpLinking,
        ServerSettingKey::AiOpenAiApiKey,
        ServerSettingKey::AiModerationOnRes,
        ServerSettingKey::AiModerationOnThread,
        ServerSettingKey::EnableSafeMode,
        ServerSettingKey::AiLlmModerationOnThread,
        ServerSettingKey::AiLlmModerationUnsafeThreads,
        ServerSettingKey::AiLlmModerationModel,
        ServerSettingKey::AiLlmModerationInstructions,
        ServerSettingKey::AiLlmModerationIntervalSeconds,
    ];

    pub const fn description(&self) -> &'static str {
        match self {
            Self::CloseNewAuthentication => {
                "Close new authentication while allowing existing users to post and re-authenticate (true/false)"
            }
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
            Self::EnableSafeMode => {
                "Enable safe mode thread filtering — hides threads with unsafe content from clients that support it (true/false)"
            }
            Self::AiLlmModerationOnThread => {
                "Check each new thread (title + body) with an LLM in batches in eddist-persistence and publish verdicts on bbs:event:thread_moderation_verdict (true/false)"
            }
            Self::AiLlmModerationUnsafeThreads => {
                "Use LLM verdicts (violations and NSFW threads) instead of the OpenAI moderation API's flagged result to decide which threads safe mode hides (true/false)"
            }
            Self::AiLlmModerationModel => "Model for LLM thread moderation (default: gpt-5.6-luna)",
            Self::AiLlmModerationInstructions => {
                "Moderation policy (system prompt) for LLM thread moderation, written against your terms of service; required, batches are dropped while it is empty"
            }
            Self::AiLlmModerationIntervalSeconds => {
                "Seconds of new threads batched into one LLM moderation request (default: 120, clamped to 10-3600)"
            }
        }
    }
}

impl std::fmt::Display for ServerSettingKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
