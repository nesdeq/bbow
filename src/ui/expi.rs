// Expi UI for BBOW - Traditional static browser interface
// A single-screen interface with integrated statistics panel
// Shows original page size vs compressed summary size

use super::{BrowserState, UIInterface, UserAction};
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
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io::{self, Stdout};
use textwrap::fill;

pub struct ExpiUI {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    scroll_position: u16,
    selected_link: usize,
    links_scroll: usize,
    max_scroll: u16,
}

// Traditional browser color scheme - optimized for dark terminals
const TEXT_PRIMARY: Color = Color::Rgb(245, 245, 245);     // Light gray text (readable on dark)
const TEXT_SECONDARY: Color = Color::Rgb(169, 169, 169);   // Medium gray for secondary text
const LINK_BLUE: Color = Color::Rgb(102, 178, 255);       // Bright blue links (visible on dark)
// const LINK_VISITED: Color = Color::Rgb(200, 100, 200);    // Light purple visited links (future use)
const BACKGROUND: Color = Color::Rgb(32, 32, 32);         // Dark background
const BORDER_GRAY: Color = Color::Rgb(128, 128, 128);     // Medium gray borders
const STATUS_BAR: Color = Color::Rgb(48, 48, 48);         // Dark gray status bar
const SUCCESS_GREEN: Color = Color::Rgb(0, 255, 127);     // Bright green for stats
const ADDRESS_BAR: Color = Color::Rgb(40, 40, 40);        // Slightly lighter dark gray

impl UIInterface for ExpiUI {
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
                // Even loading uses the same static interface
                self.terminal.draw(|f| {
                    Self::render_static_browser(
                        f,
                        url,
                        "Loading...",
                        &format!("Loading: {}% - {}", progress, stage),
                        &[],
                        self.scroll_position,
                        self.selected_link,
                        self.links_scroll,
                        None, // No stats during loading
                    )
                })?;
            }
            BrowserState::Page {
                url,
                title,
                summary,
                links,
            } => {
                // Calculate page statistics
                let original_size = summary.len();
                let compressed_size = summary.split_whitespace().count() * 5; // Rough estimate
                let stats = PageStats {
                    original_size,
                    compressed_size,
                    compression_ratio: if original_size > 0 {
                        (original_size as f32 - compressed_size as f32) / original_size as f32 * 100.0
                    } else {
                        0.0
                    },
                    link_count: links.len(),
                };

                self.terminal.draw(|f| {
                    Self::render_static_browser(
                        f,
                        url,
                        title,
                        summary,
                        links,
                        self.scroll_position,
                        self.selected_link,
                        self.links_scroll,
                        Some(&stats),
                    )
                })?;

                self.update_max_scroll(summary);
                self.update_links_scroll_with_height(15); // Fixed height for links area
            }
            BrowserState::URLInput { input } => {
                self.terminal.draw(|f| {
                    Self::render_static_browser(
                        f,
                        input,
                        "Enter URL",
                        "Type a URL and press Enter to navigate",
                        &[],
                        0,
                        0,
                        0,
                        None,
                    )
                })?;
            }
            BrowserState::URLSuggestions {
                original_url,
                error_message,
                suggestions,
                selected_index: _,
            } => {
                let suggestion_text = format!(
                    "Error: {}\n\nSuggestions:\n{}",
                    error_message,
                    suggestions.join("\n")
                );
                self.terminal.draw(|f| {
                    Self::render_static_browser(
                        f,
                        original_url,
                        "Navigation Error",
                        &suggestion_text,
                        &[],
                        0,
                        0,
                        0,
                        None,
                    )
                })?;
            }
            BrowserState::History { entries, current_index: _ } => {
                let history_text = entries
                    .iter()
                    .enumerate()
                    .map(|(i, entry)| format!("{}. {} - {}", i + 1, entry.title, entry.url))
                    .collect::<Vec<_>>()
                    .join("\n");
                
                self.terminal.draw(|f| {
                    Self::render_static_browser(
                        f,
                        "chrome://history",
                        "Browse History",
                        &history_text,
                        &[],
                        0,
                        0,
                        0,
                        None,
                    )
                })?;
            }
            BrowserState::Error { message } => {
                self.terminal.draw(|f| {
                    Self::render_static_browser(
                        f,
                        "about:error",
                        "Error",
                        &format!("An error occurred:\n\n{}", message),
                        &[],
                        0,
                        0,
                        0,
                        None,
                    )
                })?;
            }
        }
        Ok(())
    }

    fn get_user_input(&mut self, state: &BrowserState) -> Result<UserAction> {
        loop {
            if let Event::Key(key) = event::read()? {
                match state {
                    BrowserState::URLInput { .. } => match key.code {
                        KeyCode::Esc => return Ok(UserAction::CancelInput),
                        KeyCode::Enter => {
                            // Get the current input from state
                            if let BrowserState::URLInput { input } = state {
                                return Ok(UserAction::ConfirmInput(input.clone()));
                            }
                        }
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

#[derive(Debug)]
struct PageStats {
    original_size: usize,
    compressed_size: usize,
    compression_ratio: f32,
    link_count: usize,
}

impl ExpiUI {
    #[allow(clippy::too_many_arguments)]
    fn render_static_browser(
        f: &mut Frame,
        url: &str,
        title: &str,
        content: &str,
        links: &[Link],
        scroll_pos: u16,
        selected_link: usize,
        links_scroll: usize,
        stats: Option<&PageStats>,
    ) {
        let area = f.size();

        // Traditional browser layout: Title bar, Address bar, Content, Status bar
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),  // Title bar
                Constraint::Length(3),  // Address bar
                Constraint::Min(10),    // Content area
                Constraint::Length(3),  // Status bar
            ])
            .split(area);

        // Title bar (like window title)
        f.render_widget(
            Paragraph::new(format!("{} - BBOW Browser", title))
                .style(Style::default().fg(TEXT_PRIMARY).bg(STATUS_BAR))
                .alignment(Alignment::Left),
            main_chunks[0],
        );

        // Address bar
        f.render_widget(
            Paragraph::new(format!("Address: {}", url))
                .style(Style::default().fg(TEXT_PRIMARY))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(BORDER_GRAY))
                        .style(Style::default().bg(ADDRESS_BAR))
                ),
            main_chunks[1],
        );

        // Content area split between main content and sidebar
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(65), // Main content
                Constraint::Percentage(35), // Sidebar (links + stats)
            ])
            .split(main_chunks[2]);

        // Main content area
        Self::render_main_content(f, content_chunks[0], content, scroll_pos);

        // Sidebar split between links and stats
        let sidebar_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(70), // Links
                Constraint::Percentage(30), // Stats
            ])
            .split(content_chunks[1]);

        Self::render_links_panel(f, sidebar_chunks[0], links, selected_link, links_scroll);
        Self::render_stats_panel(f, sidebar_chunks[1], stats);

        // Status bar
        Self::render_status_bar(f, main_chunks[3], content, links);
    }

    fn render_main_content(f: &mut Frame, area: Rect, content: &str, scroll_pos: u16) {
        let width = area.width.saturating_sub(4) as usize;
        let visible_height = area.height.saturating_sub(2) as usize;

        let visible_lines = ui_common::get_visible_markdown_lines(
            content,
            width,
            scroll_pos,
            visible_height,
            Self::style_markdown_element,
        );

        f.render_widget(
            Paragraph::new(visible_lines)
                .style(Style::default().fg(TEXT_PRIMARY))
                .wrap(Wrap { trim: false })
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(BORDER_GRAY))
                        .title("Content")
                        .title_style(Style::default().fg(TEXT_SECONDARY)),
                ),
            area,
        );
    }

    fn render_links_panel(
        f: &mut Frame,
        area: Rect,
        links: &[Link],
        selected_link: usize,
        links_scroll: usize,
    ) {
        if links.is_empty() {
            f.render_widget(
                Paragraph::new("No links found")
                    .style(Style::default().fg(TEXT_SECONDARY))
                    .alignment(Alignment::Center)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(BORDER_GRAY))
                            .title("Links")
                            .title_style(Style::default().fg(TEXT_SECONDARY)),
                    ),
                area,
            );
            return;
        }

        let visible_height = area.height.saturating_sub(2) as usize;
        let start_index = links_scroll;
        let end_index = (start_index + visible_height).min(links.len());

        let items: Vec<ListItem> = links[start_index..end_index]
            .iter()
            .enumerate()
            .map(|(i, link)| {
                let absolute_index = start_index + i;
                let is_selected = absolute_index == selected_link;

                let style = if is_selected {
                    Style::default()
                        .fg(BACKGROUND)
                        .bg(LINK_BLUE)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(LINK_BLUE).add_modifier(Modifier::UNDERLINED)
                };

                let content = format!("[{}] {}", link.index, link.text);
                let wrapped_content = fill(&content, area.width.saturating_sub(6) as usize);
                ListItem::new(wrapped_content).style(style)
            })
            .collect();

        f.render_widget(
            List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(BORDER_GRAY))
                        .title("Links")
                        .title_style(Style::default().fg(TEXT_SECONDARY)),
                ),
            area,
        );
    }

    fn render_stats_panel(f: &mut Frame, area: Rect, stats: Option<&PageStats>) {
        let content = if let Some(stats) = stats {
            vec![
                Line::from(vec![
                    Span::styled("Page Size: ", Style::default().fg(TEXT_SECONDARY)),
                    Span::styled(
                        format!("{} bytes", stats.original_size),
                        Style::default().fg(TEXT_PRIMARY),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Summary: ", Style::default().fg(TEXT_SECONDARY)),
                    Span::styled(
                        format!("{} bytes", stats.compressed_size),
                        Style::default().fg(TEXT_PRIMARY),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Compression: ", Style::default().fg(TEXT_SECONDARY)),
                    Span::styled(
                        format!("{:.1}%", stats.compression_ratio),
                        Style::default().fg(SUCCESS_GREEN).add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Links Found: ", Style::default().fg(TEXT_SECONDARY)),
                    Span::styled(
                        format!("{}", stats.link_count),
                        Style::default().fg(TEXT_PRIMARY),
                    ),
                ]),
            ]
        } else {
            vec![
                Line::from(Span::styled(
                    "No statistics available",
                    Style::default().fg(TEXT_SECONDARY),
                )),
            ]
        };

        f.render_widget(
            Paragraph::new(content)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(BORDER_GRAY))
                        .title("Statistics")
                        .title_style(Style::default().fg(TEXT_SECONDARY)),
                ),
            area,
        );
    }

    fn render_status_bar(f: &mut Frame, area: Rect, content: &str, links: &[Link]) {
        let word_count = content.split_whitespace().count();
        let char_count = content.len();
        
        let status_text = vec![
            Line::from(vec![
                Span::styled("Ready", Style::default().fg(SUCCESS_GREEN)),
                Span::raw("  |  "),
                Span::styled(
                    format!("{} words, {} chars", word_count, char_count),
                    Style::default().fg(TEXT_SECONDARY),
                ),
                Span::raw("  |  "),
                Span::styled(
                    format!("{} links", links.len()),
                    Style::default().fg(TEXT_SECONDARY),
                ),
                Span::raw("  |  "),
                Span::styled("q:Quit g:URL h:History", Style::default().fg(TEXT_SECONDARY)),
            ]),
        ];

        f.render_widget(
            Paragraph::new(status_text)
                .style(Style::default().fg(TEXT_SECONDARY).bg(STATUS_BAR))
                .block(
                    Block::default()
                        .borders(Borders::TOP)
                        .border_style(Style::default().fg(BORDER_GRAY))
                ),
            area,
        );
    }

    fn style_markdown_element(element: &MarkdownElement) -> Style {
        match element {
            MarkdownElement::Header1(_) => Style::default()
                .fg(TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            MarkdownElement::Header2(_) => Style::default()
                .fg(TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
            MarkdownElement::Header3(_) => Style::default()
                .fg(TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
            MarkdownElement::Header4(_) => Style::default()
                .fg(TEXT_SECONDARY)
                .add_modifier(Modifier::BOLD),
            MarkdownElement::Bold(_) => Style::default()
                .fg(TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
            MarkdownElement::Italic(_) => Style::default()
                .fg(TEXT_SECONDARY)
                .add_modifier(Modifier::ITALIC),
            MarkdownElement::Code(_) => Style::default()
                .fg(TEXT_PRIMARY)
                .bg(ADDRESS_BAR),
            MarkdownElement::Normal(_) => Style::default().fg(TEXT_PRIMARY),
            MarkdownElement::Empty => Style::default(),
        }
    }

    fn update_links_scroll(&mut self) {
        self.update_links_scroll_with_height(15);
    }

    fn update_links_scroll_with_height(&mut self, visible_height: usize) {
        self.links_scroll =
            ui_common::update_links_scroll(self.selected_link, self.links_scroll, visible_height);
    }

    fn update_max_scroll(&mut self, content: &str) {
        let terminal_size = self
            .terminal
            .size()
            .unwrap_or(ratatui::layout::Rect::new(0, 0, 80, 24));

        // Calculate content area dimensions
        let content_width = terminal_size.width * 65 / 100; // 65% for main content
        let width = content_width.saturating_sub(4) as usize;
        let content_height = terminal_size.height.saturating_sub(1 + 3 + 3); // title + address + status
        let visible_height = content_height.saturating_sub(2) as usize;

        self.max_scroll = ui_common::calculate_max_scroll_for_markdown(
            content,
            width,
            visible_height,
            Self::style_markdown_element,
        );
    }
}