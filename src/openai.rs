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
    
    pub async fn suggest_urls(&self, failed_url: &str, error_message: &str) -> Result<Vec<String>> {
        let prompt = format!(
            "The user tried to access '{}' but got error: {}. Please suggest 5 most likely COMPLETE URLs they probably meant to access. Each URL must be a valid, complete URL with protocol and domain (e.g., https://www.example.com). Consider common typos, missing protocols, popular websites, and logical alternatives. For single words like 'wired', suggest the actual website like 'https://www.wired.com'. Respond with ONLY a JSON array of complete URL strings, no other text or explanation.",
            failed_url, error_message
        );
        
        let request = OpenAIRequest {
            model: OPENAI_MODEL.to_string(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: "You are a helpful URL suggestion assistant. Always respond with valid JSON array of URL strings.".to_string(),
                },
                Message {
                    role: "user".to_string(),
                    content: prompt,
                }
            ],
            max_tokens: 200,
            temperature: 0.3,
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
        
        let suggestion_text = openai_response
            .choices
            .first()
            .map(|choice| choice.message.content.clone())
            .ok_or_else(|| anyhow!("No response from OpenAI"))?;
            
        // Parse JSON response
        let suggestions: Vec<String> = serde_json::from_str(&suggestion_text)
            .map_err(|e| anyhow!("Failed to parse URL suggestions as JSON: {}", e))?;
            
        // Validate URLs and filter out invalid ones
        let valid_suggestions: Vec<String> = suggestions
            .into_iter()
            .filter(|url| Self::is_valid_url(url))
            .take(5)
            .collect();
            
        Ok(valid_suggestions)
    }
    
    fn is_valid_url(url: &str) -> bool {
        use url::Url;
        
        // Must start with http or https
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return false;
        }
        
        // Must be parseable as a valid URL
        if let Ok(parsed) = Url::parse(url) {
            // Must have a host
            if parsed.host().is_none() {
                return false;
            }
            
            // Host must contain a dot (domain.tld)
            if let Some(host) = parsed.host_str() {
                if !host.contains('.') {
                    return false;
                }
                // Host should not be just a protocol or empty
                if host.is_empty() || host.len() < 3 {
                    return false;
                }
            }
            
            true
        } else {
            false
        }
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