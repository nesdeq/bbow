// Jony Ive-inspired UI for BBOW
// Embodying principles of simplicity, elegance, and focus on content

use super::{BrowserState, HistoryEntry, UIInterface, UserAction};
use crate::common::{markdown::MarkdownElement, ui as ui_common};
use crate::links::Link;
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io::{self, Stdout};
use textwrap::fill;

pub struct JonyUI {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    scroll_position: u16,
    selected_link: usize,
    links_scroll: usize,
    max_scroll: u16,
}

// Jony Ive color palette - optimized for dark terminals
const CONTENT: Color = Color::Rgb(245, 245, 247); // Off-white for primary text
const SECONDARY: Color = Color::Rgb(174, 174, 178); // Warm gray for secondary text
const ACCENT: Color = Color::Rgb(0, 122, 255); // Apple blue (unchanged - perfect)
const SUBTLE: Color = Color::Rgb(99, 99, 102); // Darker gray for subtle elements
const DIVIDER: Color = Color::Rgb(72, 72, 74); // Dark gray for dividers
const EMPHASIS: Color = Color::Rgb(255, 255, 255); // Pure white for emphasis

impl UIInterface for JonyUI {
    fn new() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self {
            terminal,
            scroll_position: 0,
            selected_link: 0,
            links_scroll: 0,
            max_scroll: 0,
        })
    }

    fn cleanup(&mut self) -> Result<()> {
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }

    fn render(&mut self, state: &BrowserState) -> Result<()> {
        match state {
            BrowserState::Loading {
                url,
                progress,
                stage,
            } => {
                let (url, progress, stage) = (url.clone(), *progress, stage.clone());
                self.terminal
                    .draw(|f| Self::render_loading(f, &url, progress, &stage))?;
            }
            BrowserState::Page {
                url,
                title,
                summary,
                links,
            } => {
                let (url, title, summary, links) =
                    (url.clone(), title.clone(), summary.clone(), links.clone());
                let (scroll_pos, selected_link, links_scroll) =
                    (self.scroll_position, self.selected_link, self.links_scroll);

                self.terminal.draw(|f| {
                    Self::render_page(
                        f,
                        &url,
                        &title,
                        &summary,
                        &links,
                        scroll_pos,
                        selected_link,
                        links_scroll,
                    );
                })?;

                self.update_max_scroll(&summary);
                self.update_links_scroll_with_height(
                    self.terminal.size()?.height.saturating_sub(10) as usize,
                );
            }
            BrowserState::History {
                entries,
                current_index,
            } => {
                let (entries, current_index) = (entries.clone(), *current_index);
                self.terminal
                    .draw(|f| Self::render_history(f, &entries, current_index))?;
            }
            BrowserState::URLInput { input } => {
                let input = input.clone();
                self.terminal.draw(|f| Self::render_url_input(f, &input))?;
            }
            BrowserState::URLSuggestions {
                original_url,
                error_message,
                suggestions,
                selected_index,
            } => {
                let (original_url, error_message, suggestions, selected_index) = (
                    original_url.clone(),
                    error_message.clone(),
                    suggestions.clone(),
                    *selected_index,
                );
                self.terminal.draw(|f| {
                    Self::render_url_suggestions(
                        f,
                        &original_url,
                        &error_message,
                        &suggestions,
                        selected_index,
                    );
                })?;
            }
            BrowserState::Error { message } => {
                let message = message.clone();
                self.terminal.draw(|f| Self::render_error(f, &message))?;
            }
        }
        Ok(())
    }

    fn get_user_input(&mut self, state: &BrowserState) -> Result<UserAction> {
        loop {
            if let Event::Key(key) = event::read()? {
                match state {
                    BrowserState::URLInput { input } => match key.code {
                        KeyCode::Esc => return Ok(UserAction::CancelInput),
                        KeyCode::Enter => return Ok(UserAction::ConfirmInput(input.clone())),
                        KeyCode::Backspace => return Ok(UserAction::Backspace),
                        KeyCode::Char(c) => return Ok(UserAction::InputChar(c)),
                        _ => continue,
                    },
                    BrowserState::History { .. } => return Ok(UserAction::GoBack),
                    BrowserState::URLSuggestions { .. } => match key.code {
                        KeyCode::Esc => return Ok(UserAction::CancelInput),
                        KeyCode::Char('q') => return Ok(UserAction::Quit),
                        KeyCode::Up => return Ok(UserAction::SelectPrevSuggestion),
                        KeyCode::Down => return Ok(UserAction::SelectNextSuggestion),
                        KeyCode::Enter => return Ok(UserAction::ConfirmSuggestion),
                        _ => continue,
                    },
                    BrowserState::Error { .. } => return Ok(UserAction::DismissError),
                    _ => match key.code {
                        KeyCode::Char('q') => return Ok(UserAction::Quit),
                        KeyCode::Char('b') => return Ok(UserAction::GoBack),
                        KeyCode::Char('f') => return Ok(UserAction::GoForward),
                        KeyCode::Char('h') => return Ok(UserAction::ShowHistory),
                        KeyCode::Char('g') => return Ok(UserAction::EnterUrl),
                        KeyCode::Char('r') => return Ok(UserAction::Refresh),
                        KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                            return Ok(UserAction::SelectPrevLink)
                        }
                        KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                            return Ok(UserAction::SelectNextLink)
                        }
                        KeyCode::Up => return Ok(UserAction::ScrollUp),
                        KeyCode::Down => return Ok(UserAction::ScrollDown),
                        KeyCode::Enter => return Ok(UserAction::FollowSelectedLink),
                        KeyCode::Char(c) if c.is_ascii_digit() => {
                            let digit = c.to_digit(10).unwrap() as usize;
                            if digit > 0 {
                                return Ok(UserAction::FollowLink(digit));
                            }
                        }
                        _ => continue,
                    },
                }
            }
        }
    }

    fn reset_scroll(&mut self) {
        self.scroll_position = 0;
        self.selected_link = 0;
        self.links_scroll = 0;
    }

    fn scroll_up(&mut self) {
        self.scroll_position = self.scroll_position.saturating_sub(1);
    }

    fn scroll_down(&mut self) {
        if self.scroll_position < self.max_scroll {
            self.scroll_position += 1;
        }
    }

    fn select_prev_link(&mut self, links_len: usize) {
        if links_len > 0 && self.selected_link > 0 {
            self.selected_link -= 1;
            self.update_links_scroll();
        }
    }

    fn select_next_link(&mut self, links_len: usize) {
        if links_len > 0 && self.selected_link < links_len - 1 {
            self.selected_link += 1;
            self.update_links_scroll();
        }
    }

    fn get_selected_link(&self) -> usize {
        self.selected_link
    }
}

impl JonyUI {
    fn render_loading(f: &mut Frame, url: &str, progress: u16, stage: &str) {
        let area = f.size();

        // Center loading screen with generous margins - Jony Ive's love of whitespace
        let main_block = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(area.height / 3), // Top spacer
                Constraint::Length(12),              // Content area
                Constraint::Min(0),                  // Bottom spacer
            ])
            .split(area);

        let content_area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(area.width / 6), // Left spacer
                Constraint::Min(0),                 // Content
                Constraint::Length(area.width / 6), // Right spacer
            ])
            .split(main_block[1]);

        let center_area = content_area[1];

        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Title
                Constraint::Length(2), // Spacer
                Constraint::Length(3), // URL
                Constraint::Length(1), // Spacer
                Constraint::Length(2), // Progress
                Constraint::Length(1), // Spacer
                Constraint::Length(1), // Status
            ])
            .split(center_area);

        // Minimal, elegant title - no borders
        f.render_widget(
            Paragraph::new("BBOW")
                .style(Style::default().fg(EMPHASIS).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center),
            sections[0],
        );

        // URL with subtle styling
        let wrapped_url = fill(url, sections[2].width as usize);
        f.render_widget(
            Paragraph::new(wrapped_url)
                .style(Style::default().fg(SECONDARY))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true }),
            sections[2],
        );

        // Clean, minimal progress bar - no borders, refined styling
        f.render_widget(
            Gauge::default()
                .gauge_style(Style::default().fg(ACCENT))
                .percent(progress)
                .use_unicode(true),
            sections[4],
        );

        // Status with refined typography
        f.render_widget(
            Paragraph::new(stage)
                .style(Style::default().fg(SUBTLE))
                .alignment(Alignment::Center),
            sections[6],
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn render_page(
        f: &mut Frame,
        url: &str,
        title: &str,
        summary: &str,
        links: &[Link],
        scroll_pos: u16,
        selected_link: usize,
        links_scroll: usize,
    ) {
        let area = f.size();

        // Main layout with generous margins
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(4), // Header
                Constraint::Min(5),    // Content
                Constraint::Length(2), // Footer
            ])
            .split(area);

        // Header layout
        Self::render_header(f, main_chunks[0], url, title);

        // Content layout - 75/25 split for content/links
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(75), Constraint::Percentage(25)])
            .split(main_chunks[1]);

        // Add subtle divider between content and links
        let content_with_margin = content_chunks[0].inner(&Margin {
            horizontal: 0,
            vertical: 0,
        });

        let links_with_margin = content_chunks[1].inner(&Margin {
            horizontal: 1,
            vertical: 0,
        });

        Self::render_summary(f, content_with_margin, summary, scroll_pos);
        Self::render_links(f, links_with_margin, links, selected_link, links_scroll);
        Self::render_footer(f, main_chunks[2]);
    }

    fn render_header(f: &mut Frame, area: Rect, url: &str, title: &str) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Length(2)])
            .split(area);

        // Clean title without borders - focus on typography
        f.render_widget(
            Paragraph::new(title)
                .style(Style::default().fg(EMPHASIS).add_modifier(Modifier::BOLD))
                .wrap(Wrap { trim: true }),
            chunks[0],
        );

        // URL with subtle color
        f.render_widget(
            Paragraph::new(url)
                .style(Style::default().fg(SECONDARY))
                .wrap(Wrap { trim: true }),
            chunks[1],
        );
    }

    fn render_summary(f: &mut Frame, area: Rect, summary: &str, scroll_pos: u16) {
        let width = area.width.saturating_sub(2) as usize;
        let visible_height = area.height as usize;

        let visible_lines = ui_common::get_visible_markdown_lines(
            summary,
            width,
            scroll_pos,
            visible_height,
            Self::style_markdown_element,
        );

        // If no content, render empty
        if visible_lines.is_empty() {
            f.render_widget(Paragraph::new("").style(Style::default().fg(CONTENT)), area);
            return;
        }

        // Clean content area without borders - Jony Ive minimalism
        f.render_widget(
            Paragraph::new(visible_lines)
                .style(Style::default().fg(CONTENT))
                .wrap(ratatui::widgets::Wrap { trim: true }),
            area,
        );

        // Subtle scroll indicator if needed
        let max_scroll = ui_common::calculate_max_scroll_for_markdown(
            summary,
            width,
            visible_height,
            Self::style_markdown_element,
        );

        if max_scroll > 0 && area.width > 0 && area.height > 1 {
            let scroll_pos_ratio = (scroll_pos as f32 / max_scroll as f32).min(1.0);
            let indicator_y_offset =
                (scroll_pos_ratio * (area.height.saturating_sub(1)) as f32) as u16;
            let indicator_y = area.y + indicator_y_offset;

            // Ensure the indicator position is within bounds
            if indicator_y < area.y + area.height && area.x + area.width > 0 {
                f.render_widget(
                    Paragraph::new("▌").style(Style::default().fg(SUBTLE)),
                    Rect {
                        x: area.x + area.width - 1,
                        y: indicator_y,
                        width: 1,
                        height: 1,
                    },
                );
            }
        }
    }

    fn style_markdown_element(element: &MarkdownElement) -> Style {
        match element {
            MarkdownElement::Header1(_) => {
                Style::default().fg(EMPHASIS).add_modifier(Modifier::BOLD)
            }
            MarkdownElement::Header2(_) => {
                Style::default().fg(EMPHASIS).add_modifier(Modifier::BOLD)
            }
            MarkdownElement::Header3(_) => {
                Style::default().fg(EMPHASIS).add_modifier(Modifier::BOLD)
            }
            MarkdownElement::Header4(_) => Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            MarkdownElement::Bold(_) => Style::default().fg(EMPHASIS).add_modifier(Modifier::BOLD),
            MarkdownElement::Italic(_) => {
                Style::default().fg(ACCENT).add_modifier(Modifier::ITALIC)
            }
            MarkdownElement::Code(_) => Style::default().fg(EMPHASIS).bg(DIVIDER),
            MarkdownElement::Normal(_) => Style::default().fg(CONTENT),
            MarkdownElement::Empty => Style::default(),
        }
    }

    fn render_links(
        f: &mut Frame,
        area: Rect,
        links: &[Link],
        selected_link: usize,
        links_scroll: usize,
    ) {
        if links.is_empty() {
            f.render_widget(
                Paragraph::new("No links available")
                    .style(Style::default().fg(SUBTLE))
                    .alignment(Alignment::Center),
                area,
            );
            return;
        }

        // Create vertical divider
        let divider_area = Rect {
            x: area.x,
            y: area.y,
            width: 1,
            height: area.height,
        };

        for y in 0..area.height {
            f.render_widget(
                Paragraph::new("│").style(Style::default().fg(DIVIDER)),
                Rect {
                    x: divider_area.x,
                    y: divider_area.y + y,
                    width: 1,
                    height: 1,
                },
            );
        }

        // Links area with padding
        let links_area = Rect {
            x: area.x + 2,
            y: area.y,
            width: area.width.saturating_sub(2),
            height: area.height,
        };

        let visible_height = links_area.height as usize;
        let start_index = links_scroll;
        let end_index = (start_index + visible_height).min(links.len());

        let items: Vec<ListItem> = links[start_index..end_index]
            .iter()
            .enumerate()
            .map(|(i, link)| {
                let absolute_index = start_index + i;
                let is_selected = absolute_index == selected_link;

                let content = if is_selected {
                    format!("▶ {}", link.text)
                } else {
                    format!("  {}", link.text)
                };

                let style = if is_selected {
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(SECONDARY)
                };

                let wrapped_content = fill(&content, links_area.width.saturating_sub(2) as usize);
                ListItem::new(wrapped_content).style(style)
            })
            .collect();

        f.render_widget(List::new(items), links_area);
    }

    fn render_footer(f: &mut Frame, area: Rect) {
        // Minimal footer with essential controls only
        let help_text = Line::from(vec![
            Span::styled("↑↓", Style::default().fg(ACCENT)),
            Span::raw(" scroll  "),
            Span::styled("⏎", Style::default().fg(ACCENT)),
            Span::raw(" follow  "),
            Span::styled("g", Style::default().fg(ACCENT)),
            Span::raw(" url  "),
            Span::styled("q", Style::default().fg(ACCENT)),
            Span::raw(" quit"),
        ]);

        f.render_widget(
            Paragraph::new(help_text)
                .style(Style::default().fg(SUBTLE))
                .alignment(Alignment::Center),
            area,
        );
    }

    fn render_history(f: &mut Frame, entries: &[HistoryEntry], current_index: Option<usize>) {
        let area = f.size();

        // Center the history with margins
        let main_area = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(area.width / 8),
                Constraint::Min(0),
                Constraint::Length(area.width / 8),
            ])
            .split(area);

        let content_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(5),    // Content
                Constraint::Length(2), // Footer
            ])
            .split(main_area[1]);

        // Clean title
        f.render_widget(
            Paragraph::new("History")
                .style(Style::default().fg(EMPHASIS).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center),
            content_area[0],
        );

        let items: Vec<ListItem> = entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let is_current = Some(i) == current_index;
                let marker = if is_current { "▶ " } else { "  " };
                let style = if is_current {
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(CONTENT)
                };

                let content = format!("{}{}", marker, entry.title);
                let wrapped_content =
                    fill(&content, content_area[1].width.saturating_sub(4) as usize);
                ListItem::new(wrapped_content).style(style)
            })
            .collect();

        f.render_widget(List::new(items), content_area[1]);

        f.render_widget(
            Paragraph::new("Press any key to return")
                .style(Style::default().fg(SUBTLE))
                .alignment(Alignment::Center),
            content_area[2],
        );
    }

    fn render_url_input(f: &mut Frame, input: &str) {
        let area = f.size();

        // Elegant centered input
        let popup_area = Rect {
            x: area.width / 4,
            y: area.height / 2 - 2,
            width: area.width / 2,
            height: 4,
        };

        f.render_widget(Clear, popup_area);

        // Minimal input field
        f.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(DIVIDER)),
            popup_area,
        );

        let input_area = popup_area.inner(&Margin {
            horizontal: 1,
            vertical: 1,
        });
        f.render_widget(
            Paragraph::new(input).style(Style::default().fg(CONTENT)),
            input_area,
        );
    }

    fn render_url_suggestions(
        f: &mut Frame,
        original_url: &str,
        error_message: &str,
        suggestions: &[String],
        selected_index: usize,
    ) {
        let area = f.size();
        let popup_area = Rect {
            x: area.width / 8,
            y: area.height / 4,
            width: area.width * 3 / 4,
            height: area.height / 2,
        };

        f.render_widget(Clear, popup_area);
        f.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(DIVIDER)),
            popup_area,
        );

        let inner = popup_area.inner(&Margin {
            horizontal: 2,
            vertical: 1,
        });
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Error
                Constraint::Length(2), // Original URL
                Constraint::Min(3),    // Suggestions
                Constraint::Length(1), // Help
            ])
            .split(inner);

        f.render_widget(
            Paragraph::new(format!("Unable to load: {}", error_message))
                .style(Style::default().fg(Color::Red)),
            chunks[0],
        );

        f.render_widget(
            Paragraph::new(original_url).style(Style::default().fg(SECONDARY)),
            chunks[1],
        );

        let suggestion_items: Vec<ListItem> = suggestions
            .iter()
            .enumerate()
            .map(|(i, suggestion)| {
                let style = if i == selected_index {
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(CONTENT)
                };
                let marker = if i == selected_index { "▶ " } else { "  " };
                ListItem::new(format!("{}{}", marker, suggestion)).style(style)
            })
            .collect();

        f.render_widget(List::new(suggestion_items), chunks[2]);

        f.render_widget(
            Paragraph::new("↑↓ Select • ⏎ Confirm • Esc Cancel")
                .style(Style::default().fg(SUBTLE))
                .alignment(Alignment::Center),
            chunks[3],
        );
    }

    fn render_error(f: &mut Frame, message: &str) {
        let area = f.size();
        let popup_area = Rect {
            x: area.width / 6,
            y: area.height / 2 - 3,
            width: area.width * 2 / 3,
            height: 6,
        };

        f.render_widget(Clear, popup_area);
        f.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red)),
            popup_area,
        );

        let inner = popup_area.inner(&Margin {
            horizontal: 2,
            vertical: 1,
        });
        f.render_widget(
            Paragraph::new(format!("{}\n\nPress any key to dismiss", message))
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true }),
            inner,
        );
    }

    fn update_links_scroll(&mut self) {
        self.update_links_scroll_with_height(10);
    }

    fn update_links_scroll_with_height(&mut self, visible_height: usize) {
        self.links_scroll =
            ui_common::update_links_scroll(self.selected_link, self.links_scroll, visible_height);
    }

    fn update_max_scroll(&mut self, summary: &str) {
        let terminal_size = self
            .terminal
            .size()
            .unwrap_or(ratatui::layout::Rect::new(0, 0, 80, 24));

        // Match render_summary calculations exactly
        let content_width = terminal_size.width.saturating_sub(2) * 75 / 100; // 75% for content area
        let width = content_width.saturating_sub(2) as usize; // same as area.width.saturating_sub(2)
        let content_height = terminal_size.height.saturating_sub(1 + 4 + 2); // margin + header + footer
        let visible_height = content_height as usize; // same as area.height

        self.max_scroll = ui_common::calculate_max_scroll_for_markdown(
            summary,
            width,
            visible_height,
            Self::style_markdown_element,
        );
    }
}
