use crate::plugin::model::HttpWhitelistEntry;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
    pub headers: HashMap<String, String>,
}

pub struct PluginHttpClient {
    whitelist: Vec<HttpWhitelistEntry>,
    client: reqwest::Client,
}

impl PluginHttpClient {
    pub fn new(whitelist: Vec<HttpWhitelistEntry>) -> Self {
        Self {
            whitelist,
            client: reqwest::Client::new(),
        }
    }

    fn check_whitelist(&self, url: &str, method: &str) -> Result<()> {
        for entry in &self.whitelist {
            if entry.is_allowed(url, method) {
                return Ok(());
            }
        }

        bail!("URL '{}' with method '{}' is not in whitelist", url, method)
    }

    pub async fn get(&self, url: &str) -> Result<HttpResponse> {
        self.check_whitelist(url, "GET")?;

        let response = self.client.get(url).send().await?;

        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body = response.text().await?;

        Ok(HttpResponse {
            status,
            body,
            headers,
        })
    }

    pub async fn post(&self, url: &str, body: &str) -> Result<HttpResponse> {
        self.check_whitelist(url, "POST")?;

        let response = self
            .client
            .post(url)
            .body(body.to_string())
            .header("Content-Type", "application/json")
            .send()
            .await?;

        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let response_body = response.text().await?;

        Ok(HttpResponse {
            status,
            body: response_body,
            headers,
        })
    }
}
