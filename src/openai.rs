use anyhow::{Result, anyhow};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

const OPENAI_MODEL: &str = "gpt-4o-mini";
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
        
        let client = Client::new();
        
        Ok(Self { client, api_key })
    }
    
    pub async fn summarize(&self, text: &str, url: &str) -> Result<String> {
        if text.trim().is_empty() {
            return Ok("No content to summarize.".to_string());
        }
        
        let prompt = format!(
            "Please provide a concise but comprehensive summary of the following web page content from {}:\n\n{}",
            url, text
        );
        
        let request = OpenAIRequest {
            model: OPENAI_MODEL.to_string(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: "You are a helpful assistant that summarizes web content. Format your response as clean markdown with appropriate headers, bullet points, **bold** text for emphasis, and *italic* text for quotes or special terms. Use ## for main sections and - for bullet points. Keep it structured and readable.".to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: prompt,
                }
            ],
            max_tokens: MAX_TOKENS,
            temperature: TEMPERATURE,
        };
        
        let response = self.client
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
}