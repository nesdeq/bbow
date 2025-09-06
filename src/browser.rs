use anyhow::Result;
use url::Url;

use crate::{
    client::WebClient,
    extractor::TextExtractor,
    history::History,
    links::{Link, LinkExtractor},
    openai::OpenAIClient,
    ui::{HistoryEntry, UIState, UserAction, UI},
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

        self.set_loading_state(normalized_url.clone(), 0, "Starting...");
        self.ui.render(&self.current_state)?;
        self.ui.reset_scroll();

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
            }
            Err(e) => self.handle_navigation_error(url, e).await?,
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
            self.current_state = UIState::URLInput {
                input: String::new(),
            };
            self.ui.render(&self.current_state)?;
        }

        loop {
            match self.ui.get_user_input(&self.current_state)? {
                UserAction::Quit => break,
                UserAction::FollowLink(index) => self.follow_link_by_index(index).await?,
                UserAction::FollowSelectedLink => self.follow_selected_link().await?,
                UserAction::GoBack => self.handle_go_back().await?,
                UserAction::GoForward => self.handle_go_forward().await?,
                UserAction::ShowHistory => self.show_history()?,
                UserAction::EnterUrl => self.enter_url_mode()?,
                UserAction::ConfirmInput(url) => {
                    if !url.is_empty() {
                        self.navigate(&url).await?;
                    }
                }
                UserAction::CancelInput => self.return_to_page()?,
                UserAction::Refresh => self.handle_refresh().await?,
                UserAction::ScrollUp => self.scroll_up()?,
                UserAction::ScrollDown => self.scroll_down()?,
                UserAction::SelectPrevLink => self.select_prev_link()?,
                UserAction::SelectNextLink => self.select_next_link()?,
                UserAction::InputChar(c) => self.handle_input_char(c)?,
                UserAction::Backspace => self.handle_backspace()?,
                UserAction::SelectPrevSuggestion => self.select_prev_suggestion()?,
                UserAction::SelectNextSuggestion => self.select_next_suggestion()?,
                UserAction::ConfirmSuggestion => self.confirm_suggestion().await?,
                UserAction::DismissError => self.dismiss_error()?,
            }
        }

        Ok(())
    }

    async fn follow_link_by_index(&mut self, index: usize) -> Result<()> {
        if let Some(link) = self.current_links.iter().find(|l| l.index == index) {
            let url = link.url.clone();
            self.navigate(&url).await?;
        }
        Ok(())
    }

    async fn follow_selected_link(&mut self) -> Result<()> {
        let selected_index = self.ui.get_selected_link();
        if let Some(link) = self.current_links.get(selected_index) {
            let url = link.url.clone();
            self.navigate(&url).await?;
        }
        Ok(())
    }

    async fn handle_go_back(&mut self) -> Result<()> {
        if matches!(
            self.current_state,
            UIState::History { .. } | UIState::Error { .. }
        ) {
            self.return_to_page_with_message("Use 'r' to refresh for summary")?;
        } else if let Some(entry) = self.history.go_back() {
            let url = entry.url.clone();
            self.navigate(&url).await?;
        }
        Ok(())
    }

    async fn handle_go_forward(&mut self) -> Result<()> {
        if let Some(entry) = self.history.go_forward() {
            let url = entry.url.clone();
            self.navigate(&url).await?;
        }
        Ok(())
    }

    fn show_history(&mut self) -> Result<()> {
        let entries: Vec<HistoryEntry> = self
            .history
            .list()
            .iter()
            .map(|e| HistoryEntry {
                url: e.url.clone(),
                title: e.title.clone(),
            })
            .collect();

        let current_index = self
            .history
            .current()
            .and_then(|current| entries.iter().position(|e| e.url == current.url));

        self.current_state = UIState::History {
            entries,
            current_index,
        };
        self.ui.render(&self.current_state)
    }

    fn enter_url_mode(&mut self) -> Result<()> {
        self.url_input.clear();
        self.current_state = UIState::URLInput {
            input: self.url_input.clone(),
        };
        self.ui.render(&self.current_state)
    }

    fn return_to_page(&mut self) -> Result<()> {
        self.return_to_page_with_message("Use 'r' to refresh for summary")
    }

    fn return_to_page_with_message(&mut self, summary: &str) -> Result<()> {
        if let Some(current) = self.history.current() {
            self.current_state = UIState::Page {
                url: current.url.clone(),
                title: current.title.clone(),
                summary: summary.to_string(),
                links: self.current_links.clone(),
            };
            self.ui.render(&self.current_state)?;
        }
        Ok(())
    }

    async fn handle_refresh(&mut self) -> Result<()> {
        if matches!(self.current_state, UIState::Error { .. }) {
            self.return_to_page()?;
        } else if let Some(url) = self.current_url.clone() {
            self.navigate(&url).await?;
        }
        Ok(())
    }

    fn scroll_up(&mut self) -> Result<()> {
        self.ui.scroll_up();
        self.ui.render(&self.current_state)
    }

    fn scroll_down(&mut self) -> Result<()> {
        self.ui.scroll_down();
        self.ui.render(&self.current_state)
    }

    fn select_prev_link(&mut self) -> Result<()> {
        self.ui.select_prev_link(self.current_links.len());
        self.ui.render(&self.current_state)
    }

    fn select_next_link(&mut self) -> Result<()> {
        self.ui.select_next_link(self.current_links.len());
        self.ui.render(&self.current_state)
    }

    fn handle_input_char(&mut self, c: char) -> Result<()> {
        self.url_input.push(c);
        self.current_state = UIState::URLInput {
            input: self.url_input.clone(),
        };
        self.ui.render(&self.current_state)
    }

    fn handle_backspace(&mut self) -> Result<()> {
        self.url_input.pop();
        self.current_state = UIState::URLInput {
            input: self.url_input.clone(),
        };
        self.ui.render(&self.current_state)
    }

    fn select_prev_suggestion(&mut self) -> Result<()> {
        if let UIState::URLSuggestions {
            original_url,
            error_message,
            suggestions,
            selected_index,
        } = &self.current_state
        {
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
        Ok(())
    }

    fn select_next_suggestion(&mut self) -> Result<()> {
        if let UIState::URLSuggestions {
            original_url,
            error_message,
            suggestions,
            selected_index,
        } = &self.current_state
        {
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
        Ok(())
    }

    async fn confirm_suggestion(&mut self) -> Result<()> {
        if let UIState::URLSuggestions {
            suggestions,
            selected_index,
            ..
        } = &self.current_state
        {
            if let Some(selected_url) = suggestions.get(*selected_index) {
                let url = selected_url.clone();
                self.navigate(&url).await?;
            }
        }
        Ok(())
    }

    fn dismiss_error(&mut self) -> Result<()> {
        if self.history.current().is_some() {
            self.return_to_page()
        } else {
            self.current_state = UIState::URLInput {
                input: String::new(),
            };
            self.ui.render(&self.current_state)
        }
    }

    async fn fetch_and_process_with_progress(
        &mut self,
        url: &str,
    ) -> Result<(String, String, Vec<Link>)> {
        self.update_loading_progress(25, "Fetching HTML content...")
            .await?;
        let html = self.client.fetch(url).await?;

        self.update_loading_progress(50, "Extracting text content...")
            .await?;
        let text = self.extractor.extract_text(&html)?;

        self.update_loading_progress(75, "Processing page structure...")
            .await?;
        let title = self.extract_title(&html);
        let links = self.link_extractor.extract_links(&html, url)?;

        self.update_loading_progress(90, "Generating AI summary...")
            .await?;
        let summary = self.generate_summary(&text, url).await;

        self.update_loading_progress(100, "Complete!").await?;

        Ok((title, summary, links))
    }

    async fn generate_summary(&self, text: &str, url: &str) -> String {
        if text.trim().is_empty() {
            return "No content found on this page.".to_string();
        }

        match self.openai.summarize(text, url).await {
            Ok(summary) => summary,
            Err(e) => format!(
                "Failed to generate summary: {}\n\nRaw text:\n{}",
                e,
                text.chars().take(1000).collect::<String>()
            ),
        }
    }

    async fn update_loading_progress(&mut self, progress: u16, stage: &str) -> Result<()> {
        if let UIState::Loading { url, .. } = &self.current_state {
            let url = url.clone();
            self.set_loading_state(url, progress, stage);
            self.ui.render(&self.current_state)?;
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        Ok(())
    }

    fn set_loading_state(&mut self, url: String, progress: u16, stage: &str) {
        self.current_state = UIState::Loading {
            url,
            progress,
            stage: stage.to_string(),
        };
    }

    async fn handle_navigation_error(&mut self, url: &str, error: anyhow::Error) -> Result<()> {
        match self.get_url_suggestions(url, &error.to_string()).await {
            Ok(suggestions) if !suggestions.is_empty() => {
                self.current_state = UIState::URLSuggestions {
                    original_url: url.to_string(),
                    error_message: error.to_string(),
                    suggestions,
                    selected_index: 0,
                };
                self.ui.render(&self.current_state)?;
            }
            _ => {
                self.current_state = UIState::Error {
                    message: format!("Failed to load page: {}", error),
                };
                self.ui.render(&self.current_state)?;
            }
        }
        Ok(())
    }

    async fn get_url_suggestions(
        &self,
        failed_url: &str,
        error_message: &str,
    ) -> Result<Vec<String>> {
        self.openai
            .suggest_urls(failed_url, error_message)
            .await
            .or_else(|_| Ok(self.generate_fallback_suggestions(failed_url)))
    }

    fn generate_fallback_suggestions(&self, failed_url: &str) -> Vec<String> {
        let clean_input = failed_url.trim();
        let url_lower = failed_url.to_lowercase();
        let mut suggestions = Vec::new();

        if url_lower.starts_with("http") {
            if !url_lower.contains("www.") {
                if let Ok(parsed) = Url::parse(&url_lower) {
                    if let Some(host) = parsed.host_str() {
                        suggestions.push(format!(
                            "{}://www.{}{}",
                            parsed.scheme(),
                            host,
                            parsed.path()
                        ));
                    }
                }
            }
        } else if !clean_input.contains('.') {
            for ext in &["com", "org", "net", "io"] {
                suggestions.push(format!("https://www.{}.{}", clean_input, ext));
                if ext == &"com" || ext == &"io" {
                    suggestions.push(format!("https://{}.{}", clean_input, ext));
                }
            }
        } else {
            for prefix in &["https://www.", "https://", "http://www.", "http://"] {
                suggestions.push(format!("{}{}", prefix, clean_input));
            }
        }

        suggestions
            .into_iter()
            .filter(|url| self.is_valid_url_format(url))
            .take(5)
            .collect()
    }

    fn is_valid_url_format(&self, url: &str) -> bool {
        if let Ok(parsed) = Url::parse(url) {
            if let Some(host) = parsed.host_str() {
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
        let with_protocol = if url.starts_with("http://") || url.starts_with("https://") {
            url.to_string()
        } else {
            format!("https://{}", url)
        };
        Ok(Url::parse(&with_protocol)?.to_string())
    }
}
