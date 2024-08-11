use std::collections::HashMap;

use crate::domain::captcha_like::{TurnstileResponse, TURNSTILE_URL};

pub struct Secret(String);

pub struct TurnstileClient {
    client: reqwest::Client,
    secret: Secret,
}

impl TurnstileClient {
    pub fn new(secret: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            secret: Secret(secret),
        }
    }

    pub async fn verify_captcha(
        &self,
        response: &str,
        ip_addr: &str,
    ) -> Result<bool, reqwest::Error> {
        let mut form_data = HashMap::new();
        form_data.insert("response", response);
        form_data.insert("remoteip", ip_addr);
        form_data.insert("secret", &self.secret.0);

        let res = self
            .client
            .post(TURNSTILE_URL)
            .header("Authorization", self.secret.0.clone())
            .form(&form_data)
            .send()
            .await?;

        let resp = res.json::<TurnstileResponse>().await?;

        Ok(resp.success)
    }
}
