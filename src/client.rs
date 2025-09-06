use anyhow::{anyhow, Result};
use reqwest::Client;
use std::time::Duration;

const USER_AGENT: &str = "bbow/0.1.0";
const REQUEST_TIMEOUT_SECS: u64 = 30;
const MAX_REDIRECTS: usize = 5;

pub struct WebClient {
    client: Client,
}

impl WebClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .redirect(reqwest::redirect::Policy::limited(MAX_REDIRECTS))
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    pub async fn fetch(&self, url: &str) -> Result<String> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to fetch {}: {}", url, e))?;

        if !response.status().is_success() {
            return Err(anyhow!("HTTP error {}: {}", response.status(), url));
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if !content_type.contains("text/html") {
            return Err(anyhow!("Not an HTML page: {}", content_type));
        }

        response
            .text()
            .await
            .map_err(|e| anyhow!("Failed to read response body: {}", e))
    }
}
