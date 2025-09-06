use anyhow::Result;
use scraper::{Html, Selector};
use std::collections::HashSet;
use url::Url;

const MIN_LINK_TEXT_LENGTH: usize = 2;
const MAX_URL_LENGTH: usize = 200;
const MAX_LINK_TEXT_LENGTH: usize = 100;

#[derive(Debug, Clone)]
pub struct Link {
    pub text: String,
    pub url: String,
    pub index: usize,
}

pub struct LinkExtractor;

impl LinkExtractor {
    pub fn new() -> Self {
        Self
    }

    pub fn extract_links(&self, html: &str, base_url: &str) -> Result<Vec<Link>> {
        let document = Html::parse_document(html);
        let link_selector = Selector::parse("a[href]").unwrap();
        let base = Url::parse(base_url)?;

        let mut links = Vec::new();
        let mut seen_urls = HashSet::new();
        let mut index = 1;

        for element in document.select(&link_selector) {
            if let Some(href) = element.value().attr("href") {
                if let Ok(absolute_url) = base.join(href) {
                    if !absolute_url.scheme().starts_with("http") {
                        continue;
                    }

                    let url_str = absolute_url.to_string();
                    if !seen_urls.insert(url_str.clone()) || url_str.len() > MAX_URL_LENGTH {
                        continue;
                    }

                    let text = self.extract_link_text(element);
                    if text.len() < MIN_LINK_TEXT_LENGTH || self.is_noise_link(&text, &url_str) {
                        continue;
                    }

                    links.push(Link {
                        text: self.clean_link_text(&text),
                        url: url_str,
                        index,
                    });

                    index += 1;
                }
            }
        }

        Ok(links)
    }

    fn extract_link_text(&self, element: scraper::ElementRef) -> String {
        const SKIP_ELEMENTS: &[&str] = &["img", "source", "video", "audio", "script", "style"];

        let mut text_parts = Vec::new();

        for node in element.descendants() {
            if let Some(elem) = node.value().as_element() {
                if SKIP_ELEMENTS.contains(&elem.name()) {
                    continue;
                }
            }

            if let Some(text_node) = node.value().as_text() {
                let text = text_node.trim();
                if !text.is_empty() && !text.starts_with('<') {
                    text_parts.push(text);
                }
            }
        }

        let combined = text_parts.join(" ").trim().to_string();
        if !combined.is_empty() && combined.len() > 1 {
            return combined;
        }

        // Try fallback attributes
        for &attr in &["title", "aria-label"] {
            if let Some(value) = element.value().attr(attr) {
                let trimmed = value.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('<') {
                    return trimmed.to_string();
                }
            }
        }

        if let Some(img) = element
            .select(&Selector::parse("img").unwrap())
            .next()
            .and_then(|img| img.value().attr("alt"))
        {
            let alt = img.trim();
            if !alt.is_empty() && alt.len() > MIN_LINK_TEXT_LENGTH {
                return format!("[Image: {}]", alt);
            }
        }

        "<no-text>".to_string()
    }

    fn is_noise_link(&self, text: &str, url: &str) -> bool {
        let trimmed_text = text.trim();

        // Quick checks first
        if text == "<no-text>" || trimmed_text.starts_with('<') || trimmed_text.len() < 2 {
            return true;
        }

        if text.len() == 1 && !text.chars().next().unwrap().is_alphanumeric() {
            return true;
        }

        // Check URL patterns (often faster than text patterns)
        let url_lower = url.to_lowercase();
        if self.is_noise_url(&url_lower) {
            return true;
        }

        // Only check text patterns if needed
        if text.len() < 20 {
            let text_lower = text.to_lowercase();
            self.contains_noise_pattern(&text_lower)
        } else {
            false
        }
    }

    fn is_noise_url(&self, url_lower: &str) -> bool {
        const IMAGE_EXTENSIONS: &[&str] = &[
            ".jpg", ".jpeg", ".png", ".gif", ".svg", ".webp", ".bmp", ".ico",
        ];

        IMAGE_EXTENSIONS.iter().any(|&ext| url_lower.ends_with(ext))
            || url_lower.starts_with("data:")
            || url_lower.starts_with("javascript:")
            || (url_lower.contains('#') && !url_lower.contains("http"))
    }

    fn contains_noise_pattern(&self, text_lower: &str) -> bool {
        const NOISE_PATTERNS: &[&str] = &[
            "skip to",
            "skip navigation",
            "accessibility",
            "terms of service",
            "privacy policy",
            "cookie policy",
            "subscribe",
            "newsletter",
            "rss",
            "atom",
            "print",
            "share",
            "tweet",
            "facebook",
            "linkedin",
            "advertisement",
            "sponsored",
            "ad",
            "ads",
            "close",
            "×",
            "✕",
            "menu",
            "toggle",
        ];

        NOISE_PATTERNS
            .iter()
            .any(|&pattern| text_lower.contains(pattern))
    }

    fn clean_link_text(&self, text: &str) -> String {
        text.trim()
            .chars()
            .take(MAX_LINK_TEXT_LENGTH)
            .collect::<String>()
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }
}
