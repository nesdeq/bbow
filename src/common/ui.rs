// Common utilities shared between UI implementations
// This reduces code duplication between different UI themes

use crate::common::markdown::{
    parse_markdown_to_structured, render_structured_to_lines, MarkdownElement,
};
use ratatui::{style::Style, text::Line};

/// Calculate scroll bounds safely to prevent crashes
pub fn calculate_scroll_bounds(
    lines_count: usize,
    visible_height: usize,
    current_scroll: u16,
) -> (usize, usize, u16) {
    let max_scroll = lines_count.saturating_sub(visible_height) as u16;
    let safe_scroll = current_scroll.min(max_scroll);
    let start_index = if lines_count == 0 {
        0
    } else {
        (safe_scroll as usize).min(lines_count - 1)
    };
    let end_index = (start_index + visible_height).min(lines_count);

    (start_index, end_index, max_scroll)
}

/// Calculate max scroll for markdown content with given dimensions
pub fn calculate_max_scroll_for_markdown<F>(
    summary: &str,
    width: usize,
    visible_height: usize,
    styler: F,
) -> u16
where
    F: Fn(&MarkdownElement) -> Style,
{
    let parsed_lines = parse_markdown_to_structured(summary, width);
    let lines = render_structured_to_lines(&parsed_lines, styler);
    lines.len().saturating_sub(visible_height) as u16
}

/// Get visible lines from markdown content with safe bounds checking
pub fn get_visible_markdown_lines<F>(
    summary: &str,
    width: usize,
    scroll_pos: u16,
    visible_height: usize,
    styler: F,
) -> Vec<Line<'static>>
where
    F: Fn(&MarkdownElement) -> Style,
{
    let parsed_lines = parse_markdown_to_structured(summary, width);
    let lines = render_structured_to_lines(&parsed_lines, styler);

    if lines.is_empty() {
        return Vec::new();
    }

    let (start_index, end_index, _) =
        calculate_scroll_bounds(lines.len(), visible_height, scroll_pos);

    if start_index < lines.len() {
        lines[start_index..end_index].to_vec()
    } else {
        Vec::new()
    }
}

/// Update links scroll position to keep selected link visible
pub fn update_links_scroll(
    selected_link: usize,
    current_scroll: usize,
    visible_height: usize,
) -> usize {
    if visible_height == 0 {
        return current_scroll;
    }

    if selected_link < current_scroll {
        selected_link
    } else if selected_link >= current_scroll + visible_height {
        selected_link + 1 - visible_height
    } else {
        current_scroll
    }
}
