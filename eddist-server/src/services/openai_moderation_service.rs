use base64::Engine;
use chacha20poly1305::{KeyInit, aead::Aead};
use eddist_core::domain::pubsub_repository::ModerationResult;
use serde::{Deserialize, Serialize};
use tracing::warn;

use super::server_settings_cache::{ServerSettingKey, get_server_setting};

pub async fn moderate(text: &str) -> Option<ModerationResult> {
    let encrypted_key = get_server_setting(ServerSettingKey::AiOpenAiApiKey).await?;
    let api_key = decrypt_api_key(&encrypted_key);
    match call_api_with_retry(&api_key, text).await {
        Ok(result) => Some(result),
        Err(e) => {
            warn!(error = %e, "OpenAI moderation API call failed after retries");
            None
        }
    }
}

async fn call_api_with_retry(api_key: &str, input: &str) -> anyhow::Result<ModerationResult> {
    let mut last_err = anyhow::anyhow!("no attempts made");
    for attempt in 0u32..3 {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(500 * (1u64 << attempt))).await;
        }
        match call_api(api_key, input).await {
            Ok(r) => return Ok(r),
            Err(e) => {
                warn!(attempt, error = %e, "OpenAI moderation API call failed, retrying");
                last_err = e;
            }
        }
    }
    Err(last_err)
}

fn decrypt_api_key(b64: &str) -> String {
    let key = std::env::var("TINKER_SECRET").unwrap();
    let key = key.as_bytes().iter().take(32).copied().collect::<Vec<u8>>();

    let ciphertext = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .unwrap();

    let plaintext = chacha20poly1305::ChaCha20Poly1305::new(
        md5::digest::generic_array::GenericArray::from_slice(&key),
    )
    .decrypt(
        chacha20poly1305::Nonce::from_slice(&[0; 12]),
        chacha20poly1305::aead::Payload {
            msg: &ciphertext,
            aad: b"",
        },
    )
    .unwrap();

    std::str::from_utf8(&plaintext).unwrap().to_string()
}

async fn call_api(api_key: &str, input: &str) -> anyhow::Result<ModerationResult> {
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

    let client = reqwest::Client::new();
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
