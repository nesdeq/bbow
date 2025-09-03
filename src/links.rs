use scraper::{Html, Selector};
use url::Url;
use anyhow::Result;

const MIN_LINK_TEXT_LENGTH: usize = 2;
const MIN_ALT_TEXT_LENGTH: usize = 2;
const MAX_NOISE_TEXT_LENGTH: usize = 20;
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
        let mut index = 1;
        
        for element in document.select(&link_selector) {
            if let Some(href) = element.value().attr("href") {
                if let Ok(absolute_url) = self.resolve_url(&base, href) {
                    // Skip non-http(s) links
                    if !absolute_url.scheme().starts_with("http") {
                        continue;
                    }
                    
                    let text = self.extract_link_text(element);
                    
                    // Skip empty or very short link text
                    if text.len() < MIN_LINK_TEXT_LENGTH {
                        continue;
                    }
                    
                    // Skip common noise links
                    if self.is_noise_link(&text, absolute_url.as_str()) {
                        continue;
                    }
                    
                    links.push(Link {
                        text: self.clean_link_text(&text),
                        url: absolute_url.to_string(),
                        index,
                    });
                    
                    index += 1;
                }
            }
        }
        
        // Remove duplicates by URL using HashSet for O(n) performance
        let mut seen_urls = std::collections::HashSet::new();
        let mut unique_links = Vec::new();
        
        for link in links {
            if seen_urls.insert(link.url.clone()) {
                unique_links.push(link);
            }
        }
        
        // Reassign indices after deduplication
        for (i, link) in unique_links.iter_mut().enumerate() {
            link.index = i + 1;
        }
        
        links = unique_links;
        
        Ok(links)
    }
    
    fn resolve_url(&self, base: &Url, href: &str) -> Result<Url> {
        Ok(base.join(href)?)
    }
    
    fn extract_link_text(&self, element: scraper::ElementRef) -> String {
        let mut text_parts = Vec::new();
        
        // Skip certain elements that shouldn't contribute to link text
        let skip_elements = ["img", "source", "video", "audio", "script", "style"];
        
        for node in element.descendants() {
            // Skip unwanted element types
            if let Some(elem) = node.value().as_element() {
                if skip_elements.contains(&elem.name()) {
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
        
        // If we have meaningful text, use it
        if !combined.is_empty() && combined.len() > 1 {
            return combined;
        }
        
        // Fallback to title or aria-label attributes
        if let Some(title) = element.value().attr("title") {
            let title = title.trim();
            if !title.is_empty() && !title.starts_with('<') {
                return title.to_string();
            }
        }
        
        if let Some(aria_label) = element.value().attr("aria-label") {
            let aria_label = aria_label.trim();
            if !aria_label.is_empty() && !aria_label.starts_with('<') {
                return aria_label.to_string();
            }
        }
        
        // As a last resort, check if this might be an image link with alt text
        if let Some(img) = element.select(&scraper::Selector::parse("img").unwrap()).next() {
            if let Some(alt) = img.value().attr("alt") {
                let alt = alt.trim();
                if !alt.is_empty() && alt.len() > MIN_ALT_TEXT_LENGTH {
                    return format!("[Image: {}]", alt);
                }
            }
        }
        
        // If all else fails, mark it for filtering
        "<no-text>".to_string()
    }
    
    fn is_noise_link(&self, text: &str, url: &str) -> bool {
        let text_lower = text.to_lowercase();
        let url_lower = url.to_lowercase();
        
        // Skip links marked as having no meaningful text
        if text == "<no-text>" {
            return true;
        }
        
        // Skip links that start with HTML tags (image/media elements without proper text)
        if text.trim().starts_with('<') {
            // Specifically filter out <source media... and <img alt... patterns
            if text.starts_with("<source media") || text.starts_with("<img alt") {
                return true;
            }
            // Also filter out other HTML-like content that shouldn't be link text
            if text.contains("<img") || text.contains("<source") || text.contains("<video") || text.contains("<audio") {
                return true;
            }
        }
        
        // Skip links with no meaningful text (empty, whitespace, or very short)
        let cleaned_text = text.trim();
        if cleaned_text.is_empty() || cleaned_text.len() < 2 {
            return true;
        }
        
        // Skip links that are just symbols or single characters
        if cleaned_text.len() == 1 && !cleaned_text.chars().next().unwrap().is_alphanumeric() {
            return true;
        }
        
        // Skip common noise patterns
        let noise_patterns = [
            "skip to", "skip navigation", "accessibility",
            "terms of service", "privacy policy", "cookie policy",
            "subscribe", "newsletter", "rss", "atom",
            "print", "share", "tweet", "facebook", "linkedin",
            "advertisement", "sponsored", "ad", "ads",
            "close", "×", "✕", "menu", "toggle",
            "link", "image", "img", "src", "alt",
        ];
        
        for pattern in &noise_patterns {
            if text_lower.contains(pattern) && text_lower.len() < MAX_NOISE_TEXT_LENGTH {
                return true;
            }
        }
        
        // Skip image file extensions in URLs
        let image_extensions = [".jpg", ".jpeg", ".png", ".gif", ".svg", ".webp", ".bmp", ".ico"];
        for ext in &image_extensions {
            if url_lower.ends_with(ext) {
                return true;
            }
        }
        
        // Skip very long URLs (likely tracking or noise)
        if url.len() > MAX_URL_LENGTH {
            return true;
        }
        
        // Skip fragment-only links
        if url_lower.contains("#") && !url_lower.contains("http") {
            return true;
        }
        
        // Skip data URLs and javascript links
        if url_lower.starts_with("data:") || url_lower.starts_with("javascript:") {
            return true;
        }
        
        false
    }
    
    fn clean_link_text(&self, text: &str) -> String {
        text.trim()
            .chars()
            .take(MAX_LINK_TEXT_LENGTH) // Limit link text length
            .collect::<String>()
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    }
}