use crate::links::Link;
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

pub struct UI {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    scroll_position: u16,
    selected_link: usize,
    links_scroll: usize,
    max_scroll: u16,
}

#[derive(Debug, Clone)]
pub enum UIState {
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

impl UI {
    pub fn new() -> Result<Self> {
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

    pub fn cleanup(&mut self) -> Result<()> {
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }

    pub fn render(&mut self, state: &UIState) -> Result<()> {
        match state {
            UIState::Loading {
                url,
                progress,
                stage,
            } => {
                let (url, progress, stage) = (url.clone(), *progress, stage.clone());
                self.terminal
                    .draw(|f| Self::render_loading(f, &url, progress, &stage))?;
            }
            UIState::Page {
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
            UIState::History {
                entries,
                current_index,
            } => {
                let (entries, current_index) = (entries.clone(), *current_index);
                self.terminal
                    .draw(|f| Self::render_history(f, &entries, current_index))?;
            }
            UIState::URLInput { input } => {
                let input = input.clone();
                self.terminal.draw(|f| Self::render_url_input(f, &input))?;
            }
            UIState::URLSuggestions {
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
            UIState::Error { message } => {
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
        let lines = Self::parse_markdown_to_lines(summary, width);

        let visible_height = area.height.saturating_sub(2) as usize;
        let max_scroll = lines.len().saturating_sub(visible_height) as u16;

        let start_index = scroll_pos as usize;
        let end_index = (start_index + visible_height).min(lines.len());
        let visible_lines = lines[start_index..end_index].to_vec();

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
                .content_length(lines.len())
                .position(scroll_pos as usize);
            f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
        }
    }

    fn parse_markdown_to_lines(markdown: &str, width: usize) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        for line in markdown.lines() {
            let line = line.trim();

            if line.is_empty() {
                lines.push(Line::from(""));
                continue;
            }

            let (prefix, text, base_style) = Self::parse_markdown_line(line);
            let formatted_lines =
                Self::parse_inline_formatting(&format!("{}{}", prefix, text), width);

            for mut formatted_line in formatted_lines {
                for span in &mut formatted_line.spans {
                    span.style = span.style.patch(base_style);
                }
                lines.push(formatted_line);
            }
        }

        lines
    }

    fn parse_markdown_line(line: &str) -> (&str, &str, Style) {
        if let Some(text) = line.strip_prefix("#### ") {
            (
                "",
                text,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
        } else if let Some(text) = line.strip_prefix("### ") {
            (
                "",
                text,
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
        } else if let Some(text) = line.strip_prefix("## ") {
            (
                "",
                text,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        } else if let Some(text) = line.strip_prefix("# ") {
            (
                "",
                text,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )
        } else if let Some(text) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            ("• ", text, Style::default())
        } else {
            ("", line, Style::default())
        }
    }

    fn parse_inline_formatting(text: &str, width: usize) -> Vec<Line<'static>> {
        let mut spans = Vec::new();
        let mut current_text = String::new();
        let mut chars = text.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '*' if chars.peek() == Some(&'*') => {
                    chars.next();
                    if !current_text.is_empty() {
                        spans.push(Span::raw(current_text.clone()));
                        current_text.clear();
                    }

                    let mut bold_text = String::new();
                    #[allow(clippy::while_let_on_iterator)]
                    while let Some(ch) = chars.next() {
                        if ch == '*' && chars.peek() == Some(&'*') {
                            chars.next();
                            break;
                        }
                        bold_text.push(ch);
                    }
                    spans.push(Span::styled(
                        bold_text,
                        Style::default()
                            .add_modifier(Modifier::BOLD)
                            .fg(Color::White),
                    ));
                }
                '*' => {
                    if !current_text.is_empty() {
                        spans.push(Span::raw(current_text.clone()));
                        current_text.clear();
                    }

                    let mut italic_text = String::new();
                    for ch in chars.by_ref() {
                        if ch == '*' {
                            break;
                        }
                        italic_text.push(ch);
                    }
                    spans.push(Span::styled(
                        italic_text,
                        Style::default()
                            .add_modifier(Modifier::ITALIC)
                            .fg(Color::Cyan),
                    ));
                }
                '`' => {
                    if !current_text.is_empty() {
                        spans.push(Span::raw(current_text.clone()));
                        current_text.clear();
                    }

                    let mut code_text = String::new();
                    for ch in chars.by_ref() {
                        if ch == '`' {
                            break;
                        }
                        code_text.push(ch);
                    }
                    spans.push(Span::styled(
                        code_text,
                        Style::default().bg(Color::DarkGray).fg(Color::White),
                    ));
                }
                _ => current_text.push(ch),
            }
        }

        if !current_text.is_empty() {
            spans.push(Span::raw(current_text));
        }

        if spans.is_empty() {
            spans.push(Span::raw(text.to_string()));
        }

        let combined_text = spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect::<Vec<_>>()
            .join("");
        let wrapped = fill(&combined_text, width);

        if wrapped.lines().count() <= 1 {
            vec![Line::from(spans)]
        } else {
            wrapped
                .lines()
                .map(|line| Line::from(line.to_string()))
                .collect()
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

    pub fn get_user_input(&mut self, state: &UIState) -> Result<UserAction> {
        loop {
            if let Event::Key(key) = event::read()? {
                match state {
                    UIState::URLInput { input } => match key.code {
                        KeyCode::Esc => return Ok(UserAction::CancelInput),
                        KeyCode::Enter => return Ok(UserAction::ConfirmInput(input.clone())),
                        KeyCode::Backspace => return Ok(UserAction::Backspace),
                        KeyCode::Char(c) => return Ok(UserAction::InputChar(c)),
                        _ => continue,
                    },
                    UIState::History { .. } => return Ok(UserAction::GoBack),
                    UIState::URLSuggestions { .. } => match key.code {
                        KeyCode::Esc => return Ok(UserAction::CancelInput),
                        KeyCode::Char('q') => return Ok(UserAction::Quit),
                        KeyCode::Up => return Ok(UserAction::SelectPrevSuggestion),
                        KeyCode::Down => return Ok(UserAction::SelectNextSuggestion),
                        KeyCode::Enter => return Ok(UserAction::ConfirmSuggestion),
                        _ => continue,
                    },
                    UIState::Error { .. } => return Ok(UserAction::DismissError),
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

    pub fn scroll_up(&mut self) {
        self.scroll_position = self.scroll_position.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        if self.scroll_position < self.max_scroll {
            self.scroll_position += 1;
        }
    }

    pub fn select_prev_link(&mut self, links_len: usize) {
        if links_len > 0 && self.selected_link > 0 {
            self.selected_link -= 1;
            self.update_links_scroll();
        }
    }

    pub fn select_next_link(&mut self, links_len: usize) {
        if links_len > 0 && self.selected_link < links_len - 1 {
            self.selected_link += 1;
            self.update_links_scroll();
        }
    }

    fn update_links_scroll(&mut self) {
        self.update_links_scroll_with_height(10);
    }

    fn update_links_scroll_with_height(&mut self, visible_height: usize) {
        if visible_height == 0 {
            return;
        }

        if self.selected_link < self.links_scroll {
            self.links_scroll = self.selected_link;
        } else if self.selected_link >= self.links_scroll + visible_height {
            self.links_scroll = self.selected_link + 1 - visible_height;
        }
    }

    fn update_max_scroll(&mut self, summary: &str) {
        let height = self.terminal.size().map(|s| s.height).unwrap_or(24);
        let summary_height = height.saturating_sub(8);
        let width =
            (self.terminal.size().map(|s| s.width).unwrap_or(80) * 60 / 100).saturating_sub(4);
        let wrapped_text = fill(summary, width as usize);
        let lines_count = wrapped_text.lines().count();
        let visible_height = summary_height as usize;
        self.max_scroll = lines_count.saturating_sub(visible_height) as u16;
    }

    pub fn get_selected_link(&self) -> usize {
        self.selected_link
    }

    pub fn reset_scroll(&mut self) {
        self.scroll_position = 0;
        self.selected_link = 0;
        self.links_scroll = 0;
    }
}
