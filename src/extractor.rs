use anyhow::Result;
use scraper::{Html, Selector};

pub struct TextExtractor;

impl TextExtractor {
    pub fn new() -> Self {
        Self
    }

    pub fn extract_text(&self, html: &str) -> Result<String> {
        let doc = Html::parse_document(html);
        let title = self.extract_title(&doc);
        let content = self.extract_main_content(&doc);

        let result = if title.is_empty() {
            content
        } else {
            format!("# {}\n\n{}", title, content)
        };

        Ok(self.clean_text(&result))
    }

    fn extract_title(&self, document: &Html) -> String {
        let title_selector = Selector::parse("title").unwrap();
        document
            .select(&title_selector)
            .next()
            .map(|el| el.text().collect::<String>())
            .unwrap_or_default()
    }

    fn extract_main_content(&self, document: &Html) -> String {
        const MAIN_SELECTORS: &[&str] = &[
            "main",
            "article",
            "[role='main']",
            ".main-content",
            "#main-content",
            ".content",
            "#content",
        ];

        // Try main content selectors first
        for &selector_str in MAIN_SELECTORS {
            if let Ok(selector) = Selector::parse(selector_str) {
                if let Some(element) = document.select(&selector).next() {
                    return self.extract_text_from_element(element);
                }
            }
        }

        // Fallback to body, then root
        if let Ok(body_selector) = Selector::parse("body") {
            if let Some(body) = document.select(&body_selector).next() {
                return self.extract_text_from_element(body);
            }
        }

        document.root_element().text().collect::<String>()
    }

    fn extract_text_from_element(&self, element: scraper::ElementRef) -> String {
        const SKIP_TAGS: &[&str] = &[
            "script", "style", "nav", "header", "footer", "aside", "noscript",
        ];

        let mut text_parts = Vec::new();

        for node in element.descendants() {
            if let Some(elem) = node.value().as_element() {
                if SKIP_TAGS.contains(&elem.name()) {
                    continue;
                }
            }

            if let Some(text_node) = node.value().as_text() {
                let text = text_node.trim();
                if !text.is_empty() {
                    text_parts.push(text.to_string());
                }
            }
        }

        text_parts.join(" ")
    }

    fn clean_text(&self, text: &str) -> String {
        // Single pass optimization: combine operations
        text.lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
            .replace('\t', " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }
}
