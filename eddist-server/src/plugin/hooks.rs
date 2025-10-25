use crate::plugin::model::PluginHook;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HookPoint {
    BeforePostThread,
    AfterPostThread,
    BeforePostResponse,
    AfterPostResponse,
}

impl From<HookPoint> for PluginHook {
    fn from(hook: HookPoint) -> Self {
        match hook {
            HookPoint::BeforePostThread => PluginHook::BeforePostThread,
            HookPoint::AfterPostThread => PluginHook::AfterPostThread,
            HookPoint::BeforePostResponse => PluginHook::BeforePostResponse,
            HookPoint::AfterPostResponse => PluginHook::AfterPostResponse,
        }
    }
}

impl HookPoint {
    pub fn function_name(&self) -> &'static str {
        match self {
            HookPoint::BeforePostThread => "before_post_thread",
            HookPoint::AfterPostThread => "after_post_thread",
            HookPoint::BeforePostResponse => "before_post_response",
            HookPoint::AfterPostResponse => "after_post_response",
        }
    }
}
