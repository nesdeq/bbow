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
    pub line_type: LineType,
}

#[derive(Debug, Clone)]
pub enum LineType {
    Header1,
    Header2,
    Header3,
    Header4,
    Bullet,
    Normal,
}

pub fn parse_markdown_to_structured(markdown: &str, width: usize) -> Vec<ParsedLine> {
    let mut parsed_lines = Vec::new();

    for line in markdown.lines() {
        let line = line.trim();

        if line.is_empty() {
            parsed_lines.push(ParsedLine {
                elements: vec![MarkdownElement::Empty],
                prefix: String::new(),
                line_type: LineType::Normal,
            });
            continue;
        }

        let (prefix, text, line_type) = parse_markdown_line_structure(line);

        // Parse inline formatting within the text
        let inline_elements = parse_inline_elements(text);

        // Store the line type for later use in rendering
        let styled_elements = inline_elements;

        // Handle text wrapping
        let combined_text = styled_elements
            .iter()
            .map(element_text)
            .collect::<Vec<_>>()
            .join("");

        if combined_text.len() > width && !prefix.is_empty() {
            // For wrapped lines with prefixes (bullets), keep the prefix only on first line
            let wrapped = fill(&combined_text, width - prefix.len());
            let mut first = true;
            for wrapped_line in wrapped.lines() {
                parsed_lines.push(ParsedLine {
                    elements: vec![MarkdownElement::Normal(wrapped_line.to_string())],
                    prefix: if first {
                        prefix.clone()
                    } else {
                        " ".repeat(prefix.len())
                    },
                    line_type: line_type.clone(),
                });
                first = false;
            }
        } else if combined_text.len() > width {
            // For wrapped lines without prefixes
            let wrapped = fill(&combined_text, width);
            for wrapped_line in wrapped.lines() {
                parsed_lines.push(ParsedLine {
                    elements: vec![MarkdownElement::Normal(wrapped_line.to_string())],
                    prefix: String::new(),
                    line_type: line_type.clone(),
                });
            }
        } else {
            parsed_lines.push(ParsedLine {
                elements: styled_elements,
                prefix: prefix.clone(),
                line_type: line_type.clone(),
            });
        }
    }

    parsed_lines
}

fn parse_markdown_line_structure(line: &str) -> (String, &str, LineType) {
    if let Some(text) = line.strip_prefix("#### ") {
        (String::new(), text, LineType::Header4)
    } else if let Some(text) = line.strip_prefix("### ") {
        (String::new(), text, LineType::Header3)
    } else if let Some(text) = line.strip_prefix("## ") {
        (String::new(), text, LineType::Header2)
    } else if let Some(text) = line.strip_prefix("# ") {
        (String::new(), text, LineType::Header1)
    } else if let Some(text) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
        ("• ".to_string(), text, LineType::Bullet)
    } else {
        (String::new(), line, LineType::Normal)
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
        if parsed_line.elements.len() == 1
            && matches!(parsed_line.elements[0], MarkdownElement::Empty)
        {
            lines.push(Line::from(""));
            continue;
        }

        let mut spans = Vec::new();

        // Add prefix if present
        if !parsed_line.prefix.is_empty() {
            spans.push(Span::styled(
                parsed_line.prefix.clone(),
                styler(&MarkdownElement::Normal(String::new())),
            ));
        }

        // Apply line-specific styling
        match &parsed_line.line_type {
            LineType::Header1 | LineType::Header2 | LineType::Header3 | LineType::Header4 => {
                let combined_text = parsed_line
                    .elements
                    .iter()
                    .map(element_text)
                    .collect::<Vec<_>>()
                    .join("");

                let header_element = match &parsed_line.line_type {
                    LineType::Header1 => MarkdownElement::Header1(String::new()),
                    LineType::Header2 => MarkdownElement::Header2(String::new()),
                    LineType::Header3 => MarkdownElement::Header3(String::new()),
                    LineType::Header4 => MarkdownElement::Header4(String::new()),
                    _ => unreachable!(), // We're in a header match arm
                };

                spans.push(Span::styled(combined_text, styler(&header_element)));
            }
            _ => {
                // For bullets and normal text, preserve individual element formatting
                for element in &parsed_line.elements {
                    let text = element_text(element).to_string();
                    let style = styler(element);
                    spans.push(Span::styled(text, style));
                }
            }
        }

        lines.push(Line::from(spans));
    }

    lines
}
