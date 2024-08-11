use std::collections::HashMap;

use crate::domain::{
    captcha_like::{TurnstileResponse, TURNSTILE_URL},
    utils::SimpleSecret,
};

pub struct TurnstileClient {
    client: reqwest::Client,
    secret: SimpleSecret,
}

impl TurnstileClient {
    pub fn new(secret: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            secret: SimpleSecret::new(&secret),
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
        form_data.insert("secret", self.secret.get());

        let res = self
            .client
            .post(TURNSTILE_URL)
            .header("Authorization", self.secret.get())
            .form(&form_data)
            .send()
            .await?;

        let resp = res.json::<TurnstileResponse>().await?;

        Ok(resp.success)
    }
}
