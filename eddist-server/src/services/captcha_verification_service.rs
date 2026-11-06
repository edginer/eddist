use std::{collections::HashMap, time::Duration};

use futures::future::join_all;
use redis::aio::ConnectionManager;

use crate::{
    domain::captcha_like::CaptchaProviderConfig,
    external::captcha_like_client::{
        CaptchaClient, CaptchaVerificationError, CaptchaVerificationOutput, create_captcha_client,
    },
};

const CAPTCHA_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Failure policy (Monocle IP fallback, captured data) stays with the callers.
#[derive(Clone)]
pub struct CaptchaVerificationService {
    redis_conn: ConnectionManager,
    http_client: reqwest::Client,
}

impl CaptchaVerificationService {
    pub fn new(redis_conn: ConnectionManager) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(CAPTCHA_REQUEST_TIMEOUT)
            .build()
            .expect("failed to build the CAPTCHA HTTP client");
        Self {
            redis_conn,
            http_client,
        }
    }

    /// Makes no provider calls, so callers can reject a request before spending
    /// its single-use CAPTCHA tokens.
    pub fn prepare(
        &self,
        configs: &[CaptchaProviderConfig],
        responses: &HashMap<String, String>,
    ) -> Result<PreparedCaptchaVerification, CaptchaVerificationRunError> {
        let mut clients = Vec::with_capacity(configs.len());
        for config in configs {
            let form_field_name = &config.widget.form_field_name;
            let Some(response) = responses.get(form_field_name) else {
                tracing::error!(
                    provider = %config.provider,
                    field = %form_field_name,
                    "captcha response not found in form"
                );
                return Err(CaptchaVerificationRunError::MissingResponse);
            };
            let client =
                create_captcha_client(config, self.redis_conn.clone(), self.http_client.clone())?;
            clients.push(PreparedClient {
                client,
                provider_type: config.provider.to_lowercase(),
                response: response.clone(),
            });
        }
        Ok(PreparedCaptchaVerification { clients })
    }
}

pub struct PreparedCaptchaVerification {
    clients: Vec<PreparedClient>,
}

struct PreparedClient {
    client: Box<dyn CaptchaClient>,
    provider_type: String,
    response: String,
}

impl PreparedCaptchaVerification {
    /// Results are in config order.
    pub async fn run(self, origin_ip: &str) -> Vec<CaptchaVerification> {
        let results = join_all(
            self.clients
                .iter()
                .map(|c| c.client.verify_captcha(&c.response, origin_ip)),
        )
        .await;

        self.clients
            .into_iter()
            .zip(results)
            .map(|(c, result)| CaptchaVerification {
                provider_type: c.provider_type,
                result,
            })
            .collect()
    }
}

pub struct CaptchaVerification {
    /// Lowercased.
    pub provider_type: String,
    pub result: Result<CaptchaVerificationOutput, CaptchaVerificationError>,
}

pub enum CaptchaVerificationRunError {
    MissingResponse,
    /// Invalid provider configuration.
    Client(CaptchaVerificationError),
}

impl From<CaptchaVerificationError> for CaptchaVerificationRunError {
    fn from(e: CaptchaVerificationError) -> Self {
        Self::Client(e)
    }
}
