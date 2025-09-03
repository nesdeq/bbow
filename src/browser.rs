use anyhow::Result;
use url::Url;

use crate::{
    client::WebClient,
    extractor::TextExtractor,
    openai::OpenAIClient,
    links::{LinkExtractor, Link},
    ui::{UI, UIState, UserAction, HistoryEntry},
    history::History,
};

pub struct Browser {
    client: WebClient,
    extractor: TextExtractor,
    openai: OpenAIClient,
    link_extractor: LinkExtractor,
    ui: UI,
    history: History,
    current_url: Option<String>,
    current_links: Vec<Link>,
    current_state: UIState,
    url_input: String,
}

impl Browser {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: WebClient::new(),
            extractor: TextExtractor::new(),
            openai: OpenAIClient::new()?,
            link_extractor: LinkExtractor::new(),
            ui: UI::new()?,
            history: History::new(),
            current_url: None,
            current_links: Vec::new(),
            current_state: UIState::Loading { 
                url: "Starting...".to_string(),
                progress: 0,
                stage: "Initializing...".to_string(),
            },
            url_input: String::new(),
        })
    }
    
    pub async fn navigate(&mut self, url: &str) -> Result<()> {
        let normalized_url = self.normalize_url(url)?;
        self.current_url = Some(normalized_url.clone());
        
        // Show loading state
        self.current_state = UIState::Loading { 
            url: normalized_url.clone(),
            progress: 0,
            stage: "Starting...".to_string(),
        };
        self.ui.render(&self.current_state)?;
        self.ui.reset_scroll();
        
        // Fetch and process the page
        match self.fetch_and_process_with_progress(&normalized_url).await {
            Ok((title, summary, links)) => {
                self.current_links = links.clone();
                self.history.add(normalized_url, title.clone());
                self.current_state = UIState::Page {
                    url: self.current_url.as_ref().unwrap().clone(),
                    title,
                    summary,
                    links,
                };
                self.ui.render(&self.current_state)?;
            },
            Err(e) => {
                // Try to get URL suggestions from AI
                match self.get_url_suggestions(url, &e.to_string()).await {
                    Ok(suggestions) if !suggestions.is_empty() => {
                        self.current_state = UIState::URLSuggestions {
                            original_url: url.to_string(),
                            error_message: e.to_string(),
                            suggestions,
                            selected_index: 0,
                        };
                        self.ui.render(&self.current_state)?;
                    }
                    _ => {
                        self.current_state = UIState::Error {
                            message: format!("Failed to load page: {}", e),
                        };
                        self.ui.render(&self.current_state)?;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    pub async fn run(&mut self) -> Result<()> {
        let result = self.main_loop().await;
        self.ui.cleanup()?;
        result
    }
    
    async fn main_loop(&mut self) -> Result<()> {
        if self.current_url.is_none() {
            // Start with URL input
            self.current_state = UIState::URLInput { input: String::new() };
            self.ui.render(&self.current_state)?;
        }
        
        loop {
            match self.ui.get_user_input(&self.current_state)? {
                UserAction::Quit => break,
                
                UserAction::FollowLink(index) => {
                    if let Some(link) = self.current_links.iter().find(|l| l.index == index) {
                        let url = link.url.clone();
                        self.navigate(&url).await?;
                    }
                },
                
                UserAction::FollowSelectedLink => {
                    let selected_index = self.ui.get_selected_link();
                    if let Some(link) = self.current_links.get(selected_index) {
                        let url = link.url.clone();
                        self.navigate(&url).await?;
                    }
                },
                
                UserAction::GoBack => {
                    if matches!(self.current_state, UIState::History { .. } | UIState::Error { .. }) {
                        // Return to page view
                        if let Some(current) = self.history.current() {
                            self.current_state = UIState::Page {
                                url: current.url.clone(),
                                title: current.title.clone(),
                                summary: "Use 'r' to refresh for summary".to_string(),
                                links: self.current_links.clone(),
                            };
                            self.ui.render(&self.current_state)?;
                        }
                    } else if let Some(entry) = self.history.go_back() {
                        let url = entry.url.clone();
                        self.navigate(&url).await?;
                    }
                },
                
                UserAction::GoForward => {
                    if let Some(entry) = self.history.go_forward() {
                        let url = entry.url.clone();
                        self.navigate(&url).await?;
                    }
                },
                
                UserAction::ShowHistory => {
                    let entries: Vec<HistoryEntry> = self.history.list()
                        .iter()
                        .map(|e| HistoryEntry {
                            url: e.url.clone(),
                            title: e.title.clone(),
                        })
                        .collect();
                    let current_index = self.history.current().and_then(|current| {
                        entries.iter().position(|e| e.url == current.url)
                    });
                    self.current_state = UIState::History { entries, current_index };
                    self.ui.render(&self.current_state)?;
                },
                
                UserAction::EnterUrl => {
                    self.url_input.clear();
                    self.current_state = UIState::URLInput { input: self.url_input.clone() };
                    self.ui.render(&self.current_state)?;
                },
                
                UserAction::ConfirmInput(url) => {
                    if !url.is_empty() {
                        self.navigate(&url).await?;
                    }
                },
                
                UserAction::CancelInput => {
                    // Return to previous state
                    if let Some(current) = self.history.current() {
                        self.current_state = UIState::Page {
                            url: current.url.clone(),
                            title: current.title.clone(),
                            summary: "Use 'r' to refresh for summary".to_string(),
                            links: self.current_links.clone(),
                        };
                        self.ui.render(&self.current_state)?;
                    }
                },
                
                UserAction::Refresh => {
                    if matches!(self.current_state, UIState::Error { .. }) {
                        // Return to page after error
                        if let Some(current) = self.history.current() {
                            self.current_state = UIState::Page {
                                url: current.url.clone(),
                                title: current.title.clone(),
                                summary: "Use 'r' to refresh for summary".to_string(),
                                links: self.current_links.clone(),
                            };
                            self.ui.render(&self.current_state)?;
                        }
                    } else if let Some(url) = &self.current_url {
                        let url = url.clone();
                        self.navigate(&url).await?;
                    }
                },
                
                UserAction::ScrollUp => {
                    self.ui.scroll_up();
                    self.ui.render(&self.current_state)?;
                },
                
                UserAction::ScrollDown => {
                    self.ui.scroll_down();
                    self.ui.render(&self.current_state)?;
                },
                
                UserAction::SelectPrevLink => {
                    self.ui.select_prev_link(self.current_links.len());
                    self.ui.render(&self.current_state)?;
                },
                
                UserAction::SelectNextLink => {
                    self.ui.select_next_link(self.current_links.len());
                    self.ui.render(&self.current_state)?;
                },
                
                UserAction::InputChar(c) => {
                    self.url_input.push(c);
                    self.current_state = UIState::URLInput { input: self.url_input.clone() };
                    self.ui.render(&self.current_state)?;
                },
                
                UserAction::Backspace => {
                    self.url_input.pop();
                    self.current_state = UIState::URLInput { input: self.url_input.clone() };
                    self.ui.render(&self.current_state)?;
                },
                
                UserAction::SelectPrevSuggestion => {
                    if let UIState::URLSuggestions { original_url, error_message, suggestions, selected_index } = &self.current_state {
                        let new_index = if *selected_index > 0 { 
                            *selected_index - 1 
                        } else { 
                            suggestions.len().saturating_sub(1) 
                        };
                        self.current_state = UIState::URLSuggestions {
                            original_url: original_url.clone(),
                            error_message: error_message.clone(),
                            suggestions: suggestions.clone(),
                            selected_index: new_index,
                        };
                        self.ui.render(&self.current_state)?;
                    }
                },
                
                UserAction::SelectNextSuggestion => {
                    if let UIState::URLSuggestions { original_url, error_message, suggestions, selected_index } = &self.current_state {
                        let new_index = if *selected_index < suggestions.len().saturating_sub(1) { 
                            *selected_index + 1 
                        } else { 
                            0 
                        };
                        self.current_state = UIState::URLSuggestions {
                            original_url: original_url.clone(),
                            error_message: error_message.clone(),
                            suggestions: suggestions.clone(),
                            selected_index: new_index,
                        };
                        self.ui.render(&self.current_state)?;
                    }
                },
                
                UserAction::ConfirmSuggestion => {
                    if let UIState::URLSuggestions { suggestions, selected_index, .. } = &self.current_state {
                        if let Some(selected_url) = suggestions.get(*selected_index) {
                            let url_to_navigate = selected_url.clone();
                            self.navigate(&url_to_navigate).await?;
                        }
                    }
                },
                
                UserAction::DismissError => {
                    // Return to previous state or URL input
                    if let Some(current) = self.history.current() {
                        self.current_state = UIState::Page {
                            url: current.url.clone(),
                            title: current.title.clone(),
                            summary: "Use 'r' to refresh for summary".to_string(),
                            links: self.current_links.clone(),
                        };
                        self.ui.render(&self.current_state)?;
                    } else {
                        self.current_state = UIState::URLInput { input: String::new() };
                        self.ui.render(&self.current_state)?;
                    }
                },
            }
        }
        
        Ok(())
    }
    
    
    async fn fetch_and_process_with_progress(&mut self, url: &str) -> Result<(String, String, Vec<Link>)> {
        // Step 1: Fetch HTML (25% progress)
        self.update_loading_progress(25, "Fetching HTML content...").await?;
        let html = self.client.fetch(url).await?;
        
        // Step 2: Extract text (50% progress)
        self.update_loading_progress(50, "Extracting text content...").await?;
        let text = self.extractor.extract_text(&html)?;
        
        // Step 3: Extract title and links (75% progress)
        self.update_loading_progress(75, "Processing page structure...").await?;
        let title = self.extract_title(&html);
        let links = self.link_extractor.extract_links(&html, url)?;
        
        // Step 4: Generate summary (90% progress)
        self.update_loading_progress(90, "Generating AI summary...").await?;
        let summary = if !text.trim().is_empty() {
            match self.openai.summarize(&text, url).await {
                Ok(summary) => summary,
                Err(e) => format!("Failed to generate summary: {}\n\nRaw text:\n{}", e, 
                    text.chars().take(1000).collect::<String>())
            }
        } else {
            "No content found on this page.".to_string()
        };
        
        // Step 5: Complete (100% progress)
        self.update_loading_progress(100, "Complete!").await?;
        
        Ok((title, summary, links))
    }
    
    async fn update_loading_progress(&mut self, progress: u16, stage: &str) -> Result<()> {
        if let UIState::Loading { url, .. } = &self.current_state {
            self.current_state = UIState::Loading {
                url: url.clone(),
                progress,
                stage: stage.to_string(),
            };
            self.ui.render(&self.current_state)?;
            
            // Small delay to show progress
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        Ok(())
    }
    
    async fn get_url_suggestions(&self, failed_url: &str, error_message: &str) -> Result<Vec<String>> {
        match self.openai.suggest_urls(failed_url, error_message).await {
            Ok(suggestions) => Ok(suggestions),
            Err(_) => {
                // Fallback to basic suggestions if AI fails
                Ok(self.generate_fallback_suggestions(failed_url))
            }
        }
    }
    
    fn generate_fallback_suggestions(&self, failed_url: &str) -> Vec<String> {
        let mut suggestions = Vec::new();
        let url_lower = failed_url.to_lowercase();
        let clean_input = failed_url.trim();
        
        // If it already has a protocol but failed, try variations
        if url_lower.starts_with("http") {
            // Try with www if missing
            if !url_lower.contains("www.") {
                if let Ok(parsed) = url::Url::parse(&url_lower) {
                    if let Some(host) = parsed.host_str() {
                        suggestions.push(format!("{}://www.{}{}", parsed.scheme(), host, parsed.path()));
                    }
                }
            }
        } else {
            // For single words or incomplete URLs, add common patterns
            if !clean_input.contains('.') {
                // Common domain extensions for single words
                suggestions.push(format!("https://www.{}.com", clean_input));
                suggestions.push(format!("https://{}.com", clean_input));
                suggestions.push(format!("https://www.{}.org", clean_input));
                suggestions.push(format!("https://www.{}.net", clean_input));
                suggestions.push(format!("https://{}.io", clean_input));
            } else {
                // Has a dot, just add protocol
                suggestions.push(format!("https://www.{}", clean_input));
                suggestions.push(format!("https://{}", clean_input));
                suggestions.push(format!("http://www.{}", clean_input));
                suggestions.push(format!("http://{}", clean_input));
            }
        }
        
        // Validate all suggestions and keep only valid ones
        suggestions
            .into_iter()
            .filter(|url| self.is_valid_url_format(url))
            .take(5)
            .collect()
    }
    
    fn is_valid_url_format(&self, url: &str) -> bool {
        use url::Url;
        
        if let Ok(parsed) = Url::parse(url) {
            if let Some(host) = parsed.host_str() {
                // Must have a dot in the host and be at least 3 chars
                host.contains('.') && host.len() >= 3
            } else {
                false
            }
        } else {
            false
        }
    }
    
    
    fn extract_title(&self, html: &str) -> String {
        use scraper::{Html, Selector};
        
        let document = Html::parse_document(html);
        let title_selector = Selector::parse("title").unwrap();
        
        document
            .select(&title_selector)
            .next()
            .map(|el| el.text().collect::<String>())
            .unwrap_or_else(|| "Untitled".to_string())
            .trim()
            .to_string()
    }
    
    fn normalize_url(&self, url: &str) -> Result<String> {
        let url = url.trim();
        
        if url.starts_with("http://") || url.starts_with("https://") {
            let parsed = Url::parse(url)?;
            Ok(parsed.to_string())
        } else {
            let with_https = format!("https://{}", url);
            let parsed = Url::parse(&with_https)?;
            Ok(parsed.to_string())
        }
    }
}