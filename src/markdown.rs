// Shared markdown parsing logic - content should be identical across all UIs
// Only styling/colors should differ between UI implementations

use ratatui::{
    style::Style,
    text::{Line, Span},
};
use textwrap::fill;

#[derive(Debug, Clone)]
pub enum MarkdownElement {
    Header1(String),
    Header2(String), 
    Header3(String),
    Header4(String),
    Bullet(String),
    Bold(String),
    Italic(String),
    Code(String),
    Normal(String),
    Empty,
}

#[derive(Debug, Clone)]
pub struct ParsedLine {
    pub elements: Vec<MarkdownElement>,
    pub prefix: String,
}

pub fn parse_markdown_to_structured(markdown: &str, width: usize) -> Vec<ParsedLine> {
    let mut parsed_lines = Vec::new();

    for line in markdown.lines() {
        let line = line.trim();

        if line.is_empty() {
            parsed_lines.push(ParsedLine {
                elements: vec![MarkdownElement::Empty],
                prefix: String::new(),
            });
            continue;
        }

        let (prefix, text, element_type) = parse_markdown_line_structure(line);
        
        // Parse inline formatting within the text
        let inline_elements = parse_inline_elements(text);
        
        // Handle text wrapping
        let combined_text = inline_elements.iter()
            .map(element_text)
            .collect::<Vec<_>>()
            .join("");
        
        if combined_text.len() > width {
            let wrapped = fill(&combined_text, width);
            for wrapped_line in wrapped.lines() {
                parsed_lines.push(ParsedLine {
                    elements: vec![MarkdownElement::Normal(wrapped_line.to_string())],
                    prefix: prefix.clone(),
                });
            }
        } else {
            parsed_lines.push(ParsedLine {
                elements: match element_type {
                    MarkdownElement::Header1(_) => vec![MarkdownElement::Header1(text.to_string())],
                    MarkdownElement::Header2(_) => vec![MarkdownElement::Header2(text.to_string())],
                    MarkdownElement::Header3(_) => vec![MarkdownElement::Header3(text.to_string())],
                    MarkdownElement::Header4(_) => vec![MarkdownElement::Header4(text.to_string())],
                    MarkdownElement::Bullet(_) => vec![MarkdownElement::Bullet(text.to_string())],
                    _ => inline_elements,
                },
                prefix: prefix.clone(),
            });
        }
    }

    parsed_lines
}

fn parse_markdown_line_structure(line: &str) -> (String, &str, MarkdownElement) {
    if let Some(text) = line.strip_prefix("#### ") {
        (String::new(), text, MarkdownElement::Header4(String::new()))
    } else if let Some(text) = line.strip_prefix("### ") {
        (String::new(), text, MarkdownElement::Header3(String::new()))
    } else if let Some(text) = line.strip_prefix("## ") {
        (String::new(), text, MarkdownElement::Header2(String::new()))
    } else if let Some(text) = line.strip_prefix("# ") {
        (String::new(), text, MarkdownElement::Header1(String::new()))
    } else if let Some(text) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
        ("• ".to_string(), text, MarkdownElement::Bullet(String::new()))
    } else {
        (String::new(), line, MarkdownElement::Normal(String::new()))
    }
}

fn parse_inline_elements(text: &str) -> Vec<MarkdownElement> {
    let mut elements = Vec::new();
    let mut current_text = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '*' if chars.peek() == Some(&'*') => {
                chars.next();
                if !current_text.is_empty() {
                    elements.push(MarkdownElement::Normal(current_text.clone()));
                    current_text.clear();
                }

                let mut bold_text = String::new();
                while let Some(ch) = chars.next() {
                    if ch == '*' && chars.peek() == Some(&'*') {
                        chars.next();
                        break;
                    }
                    bold_text.push(ch);
                }
                elements.push(MarkdownElement::Bold(bold_text));
            }
            '*' => {
                if !current_text.is_empty() {
                    elements.push(MarkdownElement::Normal(current_text.clone()));
                    current_text.clear();
                }

                let mut italic_text = String::new();
                for ch in chars.by_ref() {
                    if ch == '*' {
                        break;
                    }
                    italic_text.push(ch);
                }
                elements.push(MarkdownElement::Italic(italic_text));
            }
            '`' => {
                if !current_text.is_empty() {
                    elements.push(MarkdownElement::Normal(current_text.clone()));
                    current_text.clear();
                }

                let mut code_text = String::new();
                for ch in chars.by_ref() {
                    if ch == '`' {
                        break;
                    }
                    code_text.push(ch);
                }
                elements.push(MarkdownElement::Code(code_text));
            }
            _ => current_text.push(ch),
        }
    }

    if !current_text.is_empty() {
        elements.push(MarkdownElement::Normal(current_text));
    }

    if elements.is_empty() {
        elements.push(MarkdownElement::Normal(text.to_string()));
    }

    elements
}

fn element_text(element: &MarkdownElement) -> &str {
    match element {
        MarkdownElement::Header1(text) => text,
        MarkdownElement::Header2(text) => text,
        MarkdownElement::Header3(text) => text,
        MarkdownElement::Header4(text) => text,
        MarkdownElement::Bullet(text) => text,
        MarkdownElement::Bold(text) => text,
        MarkdownElement::Italic(text) => text,
        MarkdownElement::Code(text) => text,
        MarkdownElement::Normal(text) => text,
        MarkdownElement::Empty => "",
    }
}

// Convenience function to convert structured markdown to ratatui Lines with custom styling
pub fn render_structured_to_lines<F>(parsed_lines: &[ParsedLine], styler: F) -> Vec<Line<'static>>
where
    F: Fn(&MarkdownElement) -> Style,
{
    let mut lines = Vec::new();

    for parsed_line in parsed_lines {
        if parsed_line.elements.len() == 1 && matches!(parsed_line.elements[0], MarkdownElement::Empty) {
            lines.push(Line::from(""));
            continue;
        }

        let mut spans = Vec::new();
        
        // Add prefix if present
        if !parsed_line.prefix.is_empty() {
            spans.push(Span::styled(parsed_line.prefix.clone(), styler(&MarkdownElement::Normal(String::new()))));
        }

        // Add elements
        for element in &parsed_line.elements {
            let text = element_text(element).to_string();
            let style = styler(element);
            spans.push(Span::styled(text, style));
        }

        lines.push(Line::from(spans));
    }

    lines
}