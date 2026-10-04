use std::collections::HashMap;

use futures::future::join_all;
use redis::aio::ConnectionManager;

use crate::{
    domain::captcha_like::CaptchaProviderConfig,
    external::captcha_like_client::{
        CaptchaLikeError, CaptchaLikeResult, CaptchaVerificationOutput, create_captcha_client,
    },
};

/// Verify captcha responses for a posting flow (thread/response creation).
///
/// Unlike `auth_with_code_service`'s verification, there is no authed token to
/// fall back on for IP equality checks (e.g. Monocle-style providers), so every
/// `Failure` variant — including `FailedToVerifyIpAddress` — is treated as a
/// hard failure here.
pub async fn verify_post_captchas(
    configs: &[CaptchaProviderConfig],
    responses: &HashMap<String, String>,
    origin_ip: &str,
    redis_conn: ConnectionManager,
) -> Result<(), CaptchaLikeError> {
    let mut clients = Vec::with_capacity(configs.len());
    for config in configs {
        let form_field_name = config.widget.form_field_name.as_str();
        if !responses.contains_key(form_field_name) {
            tracing::error!(
                provider = %config.provider,
                field = %form_field_name,
                "captcha response not found in form"
            );
            return Err(CaptchaLikeError::FailedToVerifyCaptcha);
        }
        clients.push(create_captcha_client(config, redis_conn.clone()));
    }

    let verifications = configs.iter().zip(clients.iter()).map(|(config, client)| {
        let response = &responses[config.widget.form_field_name.as_str()];
        client.verify_captcha(response, origin_ip)
    });
    let results = join_all(verifications).await;

    for (config, result) in configs.iter().zip(results) {
        match result {
            Ok(CaptchaVerificationOutput {
                result: CaptchaLikeResult::Success,
                captured_data,
                provider,
            }) => {
                if let Some(data) = captured_data {
                    tracing::debug!(provider = %provider, captured_data = ?data, "captcha verification succeeded");
                }
            }
            Ok(CaptchaVerificationOutput {
                result: CaptchaLikeResult::Failure(e),
                captured_data,
                provider,
            }) => {
                if let Some(data) = captured_data {
                    tracing::debug!(provider = %provider, captured_data = ?data, "captcha verification failed");
                }
                tracing::warn!(provider = %provider, error = %e, "captcha verification failed");
                return Err(e);
            }
            Err(e) => {
                tracing::error!(provider = %config.provider, error = %e, "captcha verification request failed");
                return Err(CaptchaLikeError::FailedToVerifyCaptcha);
            }
        }
    }

    Ok(())
}
