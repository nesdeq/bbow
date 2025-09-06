use crate::links::Link;
use crate::markdown::MarkdownElement;
use crate::ui_common;
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Wrap,
    },
    Frame, Terminal,
};
use std::io::{self, Stdout};
use textwrap::fill;

// UI Abstraction trait for decoupling browser from specific UI implementations
pub trait UIInterface {
    fn new() -> Result<Self>
    where
        Self: Sized;
    fn cleanup(&mut self) -> Result<()>;
    fn render(&mut self, state: &BrowserState) -> Result<()>;
    fn get_user_input(&mut self, state: &BrowserState) -> Result<UserAction>;
    fn reset_scroll(&mut self);
    fn scroll_up(&mut self);
    fn scroll_down(&mut self);
    fn select_prev_link(&mut self, links_len: usize);
    fn select_next_link(&mut self, links_len: usize);
    fn get_selected_link(&self) -> usize;
}

// Browser state abstraction - decoupled from specific UI implementation
#[derive(Debug, Clone)]
pub enum BrowserState {
    Loading {
        url: String,
        progress: u16,
        stage: String,
    },
    Page {
        url: String,
        title: String,
        summary: String,
        links: Vec<Link>,
    },
    History {
        entries: Vec<HistoryEntry>,
        current_index: Option<usize>,
    },
    URLInput {
        input: String,
    },
    URLSuggestions {
        original_url: String,
        error_message: String,
        suggestions: Vec<String>,
        selected_index: usize,
    },
    Error {
        message: String,
    },
}

pub struct UI {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    scroll_position: u16,
    selected_link: usize,
    links_scroll: usize,
    max_scroll: u16,
}

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub url: String,
    pub title: String,
}

#[derive(Debug)]
pub enum UserAction {
    FollowLink(usize),
    GoBack,
    GoForward,
    ShowHistory,
    EnterUrl,
    Refresh,
    Quit,
    ScrollUp,
    ScrollDown,
    SelectPrevLink,
    SelectNextLink,
    FollowSelectedLink,
    ConfirmInput(String),
    CancelInput,
    InputChar(char),
    Backspace,
    SelectPrevSuggestion,
    SelectNextSuggestion,
    ConfirmSuggestion,
    DismissError,
}

impl UIInterface for UI {
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
        self.render_internal(state)
    }

    fn get_user_input(&mut self, state: &BrowserState) -> Result<UserAction> {
        self.get_user_input_internal(state)
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

impl UI {
    // Legacy constructor for backward compatibility
    pub fn new() -> Result<Self> {
        UIInterface::new()
    }

    fn render_internal(&mut self, state: &BrowserState) -> Result<()> {
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
                    self.terminal.size()?.height.saturating_sub(8) as usize,
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

    fn render_loading(f: &mut Frame, url: &str, progress: u16, stage: &str) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(4),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(f.size());

        f.render_widget(
            Paragraph::new("🌐 Loading...")
                .style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .block(Block::default().borders(Borders::ALL).title("BBOW Browser")),
            chunks[0],
        );

        f.render_widget(
            Paragraph::new(fill(url, chunks[1].width.saturating_sub(4) as usize))
                .style(Style::default().fg(Color::Blue))
                .wrap(Wrap { trim: true })
                .block(Block::default().borders(Borders::ALL).title("URL")),
            chunks[1],
        );

        f.render_widget(
            Gauge::default()
                .block(Block::default().borders(Borders::ALL).title("Progress"))
                .gauge_style(Style::default().fg(Color::Cyan).bg(Color::Black))
                .percent(progress)
                .label(format!("{}%", progress))
                .use_unicode(true),
            chunks[2],
        );

        f.render_widget(
            Paragraph::new(stage.to_string())
                .style(Style::default().fg(Color::Yellow))
                .block(Block::default().borders(Borders::ALL).title("Status")),
            chunks[3],
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
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),
                Constraint::Min(10),
                Constraint::Length(3),
            ])
            .split(f.size());

        Self::render_header(f, main_chunks[0], url, title);

        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
            .split(main_chunks[1]);

        Self::render_summary(f, content_chunks[0], summary, scroll_pos);
        Self::render_links(f, content_chunks[1], links, selected_link, links_scroll);
        Self::render_help(f, main_chunks[2]);
    }

    fn render_header(f: &mut Frame, area: Rect, url: &str, title: &str) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Length(2)])
            .split(area);

        f.render_widget(
            Paragraph::new(format!("🌐 {}", title))
                .style(
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )
                .wrap(Wrap { trim: true })
                .block(Block::default().borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)),
            chunks[0],
        );

        f.render_widget(
            Paragraph::new(format!("📍 {}", url))
                .style(Style::default().fg(Color::Blue))
                .wrap(Wrap { trim: true })
                .block(Block::default().borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)),
            chunks[1],
        );
    }

    fn render_summary(f: &mut Frame, area: Rect, summary: &str, scroll_pos: u16) {
        let width = area.width.saturating_sub(4) as usize;
        let visible_height = area.height.saturating_sub(2) as usize;

        let visible_lines = ui_common::get_visible_markdown_lines(
            summary,
            width,
            scroll_pos,
            visible_height,
            Self::style_markdown_element,
        );

        let max_scroll = ui_common::calculate_max_scroll_for_markdown(
            summary,
            width,
            visible_height,
            Self::style_markdown_element,
        );

        let content_length = visible_lines.len() + max_scroll as usize;

        f.render_widget(
            Paragraph::new(visible_lines)
                .wrap(Wrap { trim: false })
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("📄 Summary (Markdown)"),
                )
                .scroll((0, 0)),
            area,
        );

        if max_scroll > 0 {
            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None);
            let mut scrollbar_state = ScrollbarState::default()
                .content_length(content_length)
                .position(scroll_pos as usize);
            f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
        }
    }

    fn style_markdown_element(element: &MarkdownElement) -> Style {
        match element {
            MarkdownElement::Header1(_) => Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            MarkdownElement::Header2(_) => Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            MarkdownElement::Header3(_) => Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
            MarkdownElement::Header4(_) => Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
            MarkdownElement::Bold(_) => Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::White),
            MarkdownElement::Italic(_) => Style::default()
                .add_modifier(Modifier::ITALIC)
                .fg(Color::Cyan),
            MarkdownElement::Code(_) => Style::default().bg(Color::DarkGray).fg(Color::White),
            MarkdownElement::Normal(_) => Style::default(),
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
                Paragraph::new("No links found")
                    .style(Style::default().fg(Color::Gray))
                    .block(Block::default().borders(Borders::ALL).title("🔗 Links")),
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
                let style = if absolute_index == selected_link {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                let content = format!("[{}] {}", link.index, link.text);
                let wrapped_content = fill(&content, area.width.saturating_sub(6) as usize);
                ListItem::new(wrapped_content).style(style)
            })
            .collect();

        f.render_widget(
            List::new(items)
                .block(Block::default().borders(Borders::ALL).title("🔗 Links"))
                .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan)),
            area,
        );

        if links.len() > visible_height {
            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None);
            let mut scrollbar_state = ScrollbarState::default()
                .content_length(links.len())
                .position(links_scroll);
            f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
        }
    }

    fn render_help(f: &mut Frame, area: Rect) {
        let help_text = vec![
            Line::from(vec![
                Span::styled(
                    "↑↓",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" Scroll  "),
                Span::styled(
                    "Shift+↑↓",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" Select Link  "),
                Span::styled(
                    "Enter",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" Follow  "),
                Span::styled(
                    "b",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" Back  "),
            ]),
            Line::from(vec![
                Span::styled(
                    "g",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" URL  "),
                Span::styled(
                    "h",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" History  "),
                Span::styled(
                    "r",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" Refresh  "),
                Span::styled(
                    "q",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" Quit"),
            ]),
        ];

        f.render_widget(
            Paragraph::new(help_text)
                .block(Block::default().borders(Borders::ALL).title("⌨️ Controls")),
            area,
        );
    }

    fn render_history(f: &mut Frame, entries: &[HistoryEntry], current_index: Option<usize>) {
        let area = f.size();

        let items: Vec<ListItem> = entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let marker = if Some(i) == current_index {
                    "➤ "
                } else {
                    "  "
                };
                let style = if Some(i) == current_index {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                let content = format!("{}{} - {}", marker, entry.title, entry.url);
                let wrapped_content = fill(&content, area.width.saturating_sub(4) as usize);
                ListItem::new(wrapped_content).style(style)
            })
            .collect();

        f.render_widget(
            List::new(items)
                .block(Block::default().borders(Borders::ALL).title("📚 History"))
                .style(Style::default().fg(Color::White)),
            area,
        );

        let help_area = Rect {
            x: area.x,
            y: area.y + area.height - 3,
            width: area.width,
            height: 3,
        };

        f.render_widget(Clear, help_area);
        f.render_widget(
            Paragraph::new("Press any key to return...")
                .style(Style::default().fg(Color::Gray))
                .block(Block::default().borders(Borders::ALL)),
            help_area,
        );
    }

    fn render_url_input(f: &mut Frame, input: &str) {
        let area = f.size();
        let popup_area = Rect {
            x: area.width / 4,
            y: area.height / 2 - 2,
            width: area.width / 2,
            height: 4,
        };

        f.render_widget(Clear, popup_area);
        f.render_widget(
            Paragraph::new(format!("🌐 {}", input))
                .style(Style::default().fg(Color::White))
                .block(Block::default().borders(Borders::ALL).title("Enter URL")),
            popup_area,
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

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(3),
            ])
            .split(popup_area);

        f.render_widget(
            Paragraph::new(format!("Failed to load: {}", error_message))
                .style(Style::default().fg(Color::Red))
                .wrap(Wrap { trim: true })
                .block(Block::default().borders(Borders::ALL).title("❌ Error")),
            chunks[0],
        );

        f.render_widget(
            Paragraph::new(format!("Original: {}", original_url))
                .style(Style::default().fg(Color::Yellow))
                .wrap(Wrap { trim: true })
                .block(Block::default().borders(Borders::ALL).title("🔗 URL")),
            chunks[1],
        );

        let suggestion_items: Vec<ListItem> = suggestions
            .iter()
            .enumerate()
            .map(|(i, suggestion)| {
                let style = if i == selected_index {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(suggestion.clone()).style(style)
            })
            .collect();

        f.render_widget(
            List::new(suggestion_items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("💡 Suggestions"),
                )
                .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan)),
            chunks[2],
        );

        f.render_widget(
            Paragraph::new("↑↓ Select • Enter Confirm • Esc Cancel • q Quit")
                .style(Style::default().fg(Color::Gray))
                .block(Block::default().borders(Borders::ALL).title("⌨️ Controls")),
            chunks[3],
        );
    }

    fn render_error(f: &mut Frame, message: &str) {
        let area = f.size();
        let popup_area = Rect {
            x: area.width / 8,
            y: area.height / 2 - 3,
            width: area.width * 3 / 4,
            height: 6,
        };

        f.render_widget(Clear, popup_area);
        f.render_widget(
            Paragraph::new(format!("{}\n\nPress any key to dismiss", message))
                .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
                .wrap(Wrap { trim: true })
                .block(Block::default().borders(Borders::ALL).title("❌ Error")),
            popup_area,
        );
    }

    fn get_user_input_internal(&mut self, state: &BrowserState) -> Result<UserAction> {
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
        let content_width = terminal_size.width * 80 / 100; // 80% for content area
        let width = content_width.saturating_sub(4) as usize; // same as area.width.saturating_sub(4)
        let main_content_height = terminal_size.height.saturating_sub(5 + 3); // header + footer
        let visible_height = main_content_height.saturating_sub(2) as usize; // same as area.height.saturating_sub(2)

        self.max_scroll = ui_common::calculate_max_scroll_for_markdown(
            summary,
            width,
            visible_height,
            Self::style_markdown_element,
        );
    }
}
