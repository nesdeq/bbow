use anyhow::Result;
use scraper::{Html, Selector};

pub struct TextExtractor;

impl TextExtractor {
    pub fn new() -> Self {
        Self
    }

    pub fn extract_text(&self, html: &str) -> Result<String> {
        let clean_html = self.remove_scripts_and_styles(html);
        let clean_doc = Html::parse_document(&clean_html);

        let title = self.extract_title(&clean_doc);
        let content = self.extract_main_content(&clean_doc);

        let mut result = String::new();
        if !title.is_empty() {
            result.push_str(&format!("# {}\n\n", title));
        }
        result.push_str(&content);

        Ok(self.clean_text(&result))
    }

    fn remove_scripts_and_styles(&self, html: &str) -> String {
        let script_selector = Selector::parse("script, style, noscript").unwrap();
        let mut clean_html = html.to_string();

        let doc = Html::parse_document(&clean_html);
        for element in doc.select(&script_selector) {
            if let Some(html_content) = element.html().get(..) {
                clean_html = clean_html.replace(html_content, "");
            }
        }

        clean_html
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

        for selector_str in MAIN_SELECTORS {
            if let Ok(selector) = Selector::parse(selector_str) {
                if let Some(main_element) = document.select(&selector).next() {
                    return self.extract_text_from_element(main_element);
                }
            }
        }

        let body_selector = Selector::parse("body").unwrap();
        document
            .select(&body_selector)
            .next()
            .map(|body| self.extract_text_from_element(body))
            .unwrap_or_else(|| document.root_element().text().collect::<String>())
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
