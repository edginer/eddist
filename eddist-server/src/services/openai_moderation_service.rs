use std::time::Duration;

use eddist_core::{domain::pubsub_repository::ModerationResult, symmetric};
use metrics::counter;
use serde::{Deserialize, Serialize};
use tracing::{error, warn};

use super::server_settings_cache::{ServerSettingKey, ServerSettingsSnapshot};

/// Per attempt: with retries and backoff one call can take ~33s, longer than the
/// background task drain deadline at shutdown.
const MODERATION_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone)]
pub struct OpenAiModerationService {
    client: reqwest::Client,
}

impl OpenAiModerationService {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(MODERATION_REQUEST_TIMEOUT)
                .build()
                .expect("failed to build the moderation HTTP client"),
        }
    }

    pub async fn moderate(
        &self,
        settings: &ServerSettingsSnapshot,
        text: &str,
    ) -> Option<ModerationResult> {
        let encrypted_key = settings.get(ServerSettingKey::AiOpenAiApiKey)?;
        let api_key = match symmetric::decrypt(encrypted_key) {
            Ok(k) => k,
            Err(e) => {
                error!(error = %e, "Failed to decrypt OpenAI API key; skipping moderation");
                return None;
            }
        };
        match call_api_with_retry(&self.client, &api_key, text).await {
            Ok(result) => {
                counter!("openai_moderation_requests", "result" => "success").increment(1);
                Some(result)
            }
            Err(e) => {
                warn!(error = %e, "OpenAI moderation API call failed after retries");
                counter!("openai_moderation_requests", "result" => "error").increment(1);
                None
            }
        }
    }
}

impl Default for OpenAiModerationService {
    fn default() -> Self {
        Self::new()
    }
}

async fn call_api_with_retry(
    client: &reqwest::Client,
    api_key: &str,
    input: &str,
) -> anyhow::Result<ModerationResult> {
    let mut last_err = anyhow::anyhow!("no attempts made");
    for attempt in 0u32..3 {
        if attempt > 0 {
            tokio::time::sleep(Duration::from_millis(500 * (1u64 << attempt))).await;
            counter!("openai_moderation_retries").increment(1);
        }
        match call_api(client, api_key, input).await {
            Ok(r) => return Ok(r),
            Err(e) => {
                warn!(attempt, error = %e, "OpenAI moderation API call failed, retrying");
                last_err = e;
            }
        }
    }
    Err(last_err)
}

async fn call_api(
    client: &reqwest::Client,
    api_key: &str,
    input: &str,
) -> anyhow::Result<ModerationResult> {
    #[derive(Serialize)]
    struct Request<'a> {
        input: &'a str,
    }

    #[derive(Deserialize)]
    struct Response {
        results: Vec<RawResult>,
    }

    #[derive(Deserialize)]
    struct RawResult {
        flagged: bool,
        categories: serde_json::Value,
        category_scores: serde_json::Value,
    }

    counter!("openai_moderation_api_calls").increment(1);

    let resp = client
        .post("https://api.openai.com/v1/moderations")
        .bearer_auth(api_key)
        .json(&Request { input })
        .send()
        .await?
        .error_for_status()?
        .json::<Response>()
        .await?;

    let raw = resp
        .results
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("empty moderation results"))?;

    Ok(ModerationResult {
        flagged: raw.flagged,
        categories: raw.categories,
        category_scores: raw.category_scores,
    })
}
