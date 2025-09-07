use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

const OPENAI_MODEL: &str = "gpt-4.1-mini";
const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";
const MAX_TOKENS: u32 = 500;
const TEMPERATURE: f32 = 0.3;

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

pub struct OpenAIClient {
    client: Client,
    api_key: String,
}

impl OpenAIClient {
    pub fn new() -> Result<Self> {
        let api_key = env::var("OPENAI_API_KEY")
            .map_err(|_| anyhow!("OPENAI_API_KEY environment variable not set"))?;

        Ok(Self {
            client: Client::new(),
            api_key,
        })
    }

    pub async fn suggest_urls(&self, failed_url: &str, error_message: &str) -> Result<Vec<String>> {
        let prompt = format!(
            "The user tried to access '{}' but got error: {}. \
            Please suggest 5 most likely COMPLETE URLs they probably meant to access. \
            Each URL must be a valid, complete URL with protocol and domain (e.g., https://www.example.com). \
            Consider common typos, missing protocols, popular websites, and logical alternatives. \
            For single words like 'wired', suggest the actual website like 'https://www.wired.com'. \
            Respond with ONLY a JSON array of complete URL strings, no other text or explanation.",
            failed_url, error_message
        );

        let response_text = self
            .call_openai(
                "You are a helpful URL suggestion assistant. Always respond with valid JSON array of URL strings.",
                &prompt,
                200,
            )
            .await?;

        let suggestions: Vec<String> = serde_json::from_str(&response_text)
            .map_err(|e| anyhow!("Failed to parse URL suggestions as JSON: {}", e))?;

        Ok(suggestions
            .into_iter()
            .filter(|url| Self::is_valid_url(url))
            .take(5)
            .collect())
    }

    pub async fn summarize(&self, text: &str, url: &str) -> Result<String> {
        if text.trim().is_empty() {
            return Ok("No content to summarize.".to_string());
        }

        let prompt = format!(
            "Please provide a concise but comprehensive summary of the following web page content from {}:\n\n{}",
            url, text
        );

        self.call_openai(
            "You are a helpful assistant that summarizes web content. \
            Format your response as clean markdown with appropriate headers, bullet points, \
            **bold** text for emphasis, and *italic* text for quotes or special terms. \
            Use ## for main sections and - for bullet points. Keep it structured and readable.",
            &prompt,
            MAX_TOKENS,
        )
        .await
    }

    async fn call_openai(
        &self,
        system_message: &str,
        user_prompt: &str,
        max_tokens: u32,
    ) -> Result<String> {
        let request = OpenAIRequest {
            model: OPENAI_MODEL.to_string(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: system_message.to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: user_prompt.to_string(),
                },
            ],
            max_tokens,
            temperature: TEMPERATURE,
        };

        let response = self
            .client
            .post(OPENAI_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send request to OpenAI: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("OpenAI API error {}: {}", status, error_text));
        }

        let openai_response: OpenAIResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse OpenAI response: {}", e))?;

        openai_response
            .choices
            .first()
            .map(|choice| choice.message.content.clone())
            .ok_or_else(|| anyhow!("No response from OpenAI"))
    }

    fn is_valid_url(url: &str) -> bool {
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return false;
        }

        if let Ok(parsed) = url::Url::parse(url) {
            if let Some(host) = parsed.host_str() {
                host.contains('.') && host.len() >= 3
            } else {
                false
            }
        } else {
            false
        }
    }
}
