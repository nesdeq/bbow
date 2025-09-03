use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
    Frame, Terminal,
};
use std::io::{self, Stdout};
use textwrap::fill;
use crate::links::Link;

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
        progress: u16,  // 0-100
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
    URLInput { input: String },
    Error { message: String },
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
            UIState::Loading { url, progress, stage } => {
                let url = url.clone();
                let progress = *progress;
                let stage = stage.clone();
                self.terminal.draw(|f| {
                    Self::render_loading_static(f, &url, progress, &stage);
                })?;
            }
            UIState::Page { url, title, summary, links } => {
                let url = url.clone();
                let title = title.clone();
                let summary = summary.clone();
                let links = links.clone();
                let scroll_pos = self.scroll_position;
                let selected_link = self.selected_link;
                let links_scroll = self.links_scroll;
                
                self.terminal.draw(|f| {
                    Self::render_page_static(f, &url, &title, &summary, &links, scroll_pos, selected_link, links_scroll);
                })?;
                
                // Update max_scroll after rendering
                let summary_height = self.terminal.size()?.height.saturating_sub(8);
                let width = (self.terminal.size()?.width * 60 / 100).saturating_sub(4);
                let wrapped_text = fill(&summary, width as usize);
                let lines_count = wrapped_text.lines().count();
                let visible_height = summary_height as usize;
                self.max_scroll = lines_count.saturating_sub(visible_height) as u16;
                
                // Update links scroll based on actual dimensions
                let links_height = self.terminal.size()?.height.saturating_sub(8);
                let links_visible_height = links_height as usize;
                self.update_links_scroll_with_height(links_visible_height);
            }
            UIState::History { entries, current_index } => {
                let entries = entries.clone();
                let current_index = *current_index;
                self.terminal.draw(|f| {
                    Self::render_history_static(f, &entries, current_index);
                })?;
            }
            UIState::URLInput { input } => {
                let input = input.clone();
                self.terminal.draw(|f| {
                    Self::render_url_input_static(f, &input);
                })?;
            }
            UIState::Error { message } => {
                let message = message.clone();
                self.terminal.draw(|f| {
                    Self::render_error_static(f, &message);
                })?;
            }
        }
        Ok(())
    }

    fn render_loading_static(f: &mut Frame, url: &str, progress: u16, stage: &str) {
        let area = f.size();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(3),   // Title
                Constraint::Length(4),   // URL
                Constraint::Length(3),   // Progress bar
                Constraint::Length(3),   // Status
            ].as_ref())
            .split(area);

        let title = Paragraph::new("🌐 Loading...")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL).title("BBOW Browser"));
        f.render_widget(title, chunks[0]);

        let url_text = fill(url, (chunks[1].width as usize).saturating_sub(4));
        let url_widget = Paragraph::new(url_text)
            .style(Style::default().fg(Color::Blue))
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title("URL"));
        f.render_widget(url_widget, chunks[1]);

        // Progress bar
        let progress_label = format!("{}%", progress);
        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Progress"))
            .gauge_style(Style::default().fg(Color::Cyan).bg(Color::Black))
            .percent(progress)
            .label(progress_label)
            .use_unicode(true);
        f.render_widget(gauge, chunks[2]);

        let status = Paragraph::new(stage.to_string())
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title("Status"));
        f.render_widget(status, chunks[3]);
    }

    fn render_page_static(f: &mut Frame, url: &str, title: &str, summary: &str, links: &[Link], scroll_pos: u16, selected_link: usize, links_scroll: usize) {
        let area = f.size();
        
        // Main layout
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),  // Header
                Constraint::Min(10),    // Content
                Constraint::Length(3),  // Help
            ].as_ref())
            .split(area);

        // Render header
        Self::render_header_static(f, main_chunks[0], url, title);

        // Content layout
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(80),  // Summary
                Constraint::Percentage(20),  // Links
            ].as_ref())
            .split(main_chunks[1]);

        // Render summary
        Self::render_summary_static(f, content_chunks[0], summary, scroll_pos);

        // Render links
        Self::render_links_static(f, content_chunks[1], links, selected_link, links_scroll);

        // Render help
        Self::render_help_static(f, main_chunks[2]);
    }

    fn render_header_static(f: &mut Frame, area: Rect, url: &str, title: &str) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Length(2)].as_ref())
            .split(area);

        let title_text = format!("🌐 {}", title);
        let title_widget = Paragraph::new(title_text)
            .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::TOP | Borders::LEFT | Borders::RIGHT));
        f.render_widget(title_widget, chunks[0]);

        let url_text = format!("📍 {}", url);
        let url_widget = Paragraph::new(url_text)
            .style(Style::default().fg(Color::Blue))
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT));
        f.render_widget(url_widget, chunks[1]);
    }

    fn render_summary_static(f: &mut Frame, area: Rect, summary: &str, scroll_pos: u16) {
        let width = (area.width as usize).saturating_sub(4);
        
        // Parse markdown and create styled lines
        let lines = Self::parse_markdown_to_lines(summary, width);

        // Calculate scrolling
        let visible_height = (area.height as usize).saturating_sub(2);
        let max_scroll = lines.len().saturating_sub(visible_height) as u16;
        
        let start_index = scroll_pos as usize;
        let end_index = (start_index + visible_height).min(lines.len());
        let visible_lines = lines[start_index..end_index].to_vec();

        let paragraph = Paragraph::new(visible_lines)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).title("📄 Summary (Markdown)"))
            .scroll((0, 0));
        f.render_widget(paragraph, area);

        // Render scrollbar if needed
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
            
            // Empty lines
            if line.is_empty() {
                lines.push(Line::from(""));
                continue;
            }
            
            // Headers with inline formatting support
            if let Some(header_text) = line.strip_prefix("#### ") {
                let formatted_lines = Self::parse_inline_formatting(header_text, width);
                for mut formatted_line in formatted_lines {
                    for span in &mut formatted_line.spans {
                        span.style = span.style.fg(Color::Cyan).add_modifier(Modifier::BOLD);
                    }
                    lines.push(formatted_line);
                }
                continue;
            }
            
            if let Some(header_text) = line.strip_prefix("### ") {
                let formatted_lines = Self::parse_inline_formatting(header_text, width);
                for mut formatted_line in formatted_lines {
                    for span in &mut formatted_line.spans {
                        span.style = span.style.fg(Color::Green).add_modifier(Modifier::BOLD);
                    }
                    lines.push(formatted_line);
                }
                continue;
            }
            
            if let Some(header_text) = line.strip_prefix("## ") {
                let formatted_lines = Self::parse_inline_formatting(header_text, width);
                for mut formatted_line in formatted_lines {
                    for span in &mut formatted_line.spans {
                        span.style = span.style.fg(Color::Yellow).add_modifier(Modifier::BOLD);
                    }
                    lines.push(formatted_line);
                }
                continue;
            }
            
            if let Some(header_text) = line.strip_prefix("# ") {
                let formatted_lines = Self::parse_inline_formatting(header_text, width);
                for mut formatted_line in formatted_lines {
                    for span in &mut formatted_line.spans {
                        span.style = span.style.fg(Color::Yellow).add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
                    }
                    lines.push(formatted_line);
                }
                continue;
            }
            
            // Bullet points with inline formatting
            if let Some(bullet_text) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
                let formatted_lines = Self::parse_inline_formatting(bullet_text, width);
                for (i, formatted_line) in formatted_lines.into_iter().enumerate() {
                    if i == 0 {
                        // Add bullet symbol at the beginning of first line
                        let mut bullet_spans = vec![Span::raw("• ")];
                        bullet_spans.extend(formatted_line.spans);
                        lines.push(Line::from(bullet_spans));
                    } else {
                        // Indent continuation lines
                        let mut indented_spans = vec![Span::raw("  ")];
                        indented_spans.extend(formatted_line.spans);
                        lines.push(Line::from(indented_spans));
                    }
                }
                continue;
            }
            
            // Parse inline formatting (bold, italic, code)
            let parsed_line = Self::parse_inline_formatting(line, width);
            lines.extend(parsed_line);
        }
        
        lines
    }
    
    fn parse_inline_formatting(text: &str, width: usize) -> Vec<Line<'static>> {
        let mut spans = Vec::new();
        let mut current_text = String::new();
        let mut chars = text.chars().peekable();
        
        while let Some(ch) = chars.next() {
            match ch {
                '*' => {
                    if chars.peek() == Some(&'*') {
                        // Bold text **text**
                        chars.next(); // consume second *
                        if !current_text.is_empty() {
                            spans.push(Span::raw(current_text.clone()));
                            current_text.clear();
                        }
                        
                        let mut bold_text = String::new();
                        while let Some(ch) = chars.next() {
                            if ch == '*' && chars.peek() == Some(&'*') {
                                chars.next(); // consume second *
                                break;
                            }
                            bold_text.push(ch);
                        }
                        spans.push(Span::styled(bold_text.to_string(), Style::default().add_modifier(Modifier::BOLD).fg(Color::White)));
                    } else {
                        // Italic text *text*
                        if !current_text.is_empty() {
                            spans.push(Span::raw(current_text.clone()));
                            current_text.clear();
                        }
                        
                        let mut italic_text = String::new();
                        while let Some(ch) = chars.next() {
                            if ch == '*' {
                                break;
                            }
                            italic_text.push(ch);
                        }
                        spans.push(Span::styled(italic_text.to_string(), Style::default().add_modifier(Modifier::ITALIC).fg(Color::Cyan)));
                    }
                }
                '`' => {
                    // Inline code `code`
                    if !current_text.is_empty() {
                        spans.push(Span::raw(current_text.clone()));
                        current_text.clear();
                    }
                    
                    let mut code_text = String::new();
                    while let Some(ch) = chars.next() {
                        if ch == '`' {
                            break;
                        }
                        code_text.push(ch);
                    }
                    spans.push(Span::styled(code_text.to_string(), Style::default().bg(Color::DarkGray).fg(Color::White)));
                }
                _ => {
                    current_text.push(ch);
                }
            }
        }
        
        if !current_text.is_empty() {
            spans.push(Span::raw(current_text));
        }
        
        // Handle empty spans (no formatting found)
        if spans.is_empty() {
            spans.push(Span::raw(text.to_string()));
        }
        
        // Wrap text to fit width
        let combined_text = spans.iter().map(|s| s.content.as_ref()).collect::<Vec<_>>().join("");
        let wrapped = fill(&combined_text, width);
        
        // For now, apply formatting to the first wrapped line only
        // TODO: Proper text wrapping with formatting preserved
        if wrapped.lines().count() <= 1 {
            vec![Line::from(spans)]
        } else {
            wrapped.lines().map(|line| Line::from(line.to_string())).collect()
        }
    }

    fn render_links_static(f: &mut Frame, area: Rect, links: &[Link], selected_link: usize, links_scroll: usize) {
        if links.is_empty() {
            let no_links = Paragraph::new("No links found")
                .style(Style::default().fg(Color::Gray))
                .block(Block::default().borders(Borders::ALL).title("🔗 Links"));
            f.render_widget(no_links, area);
            return;
        }

        let visible_height = (area.height as usize).saturating_sub(2); // Account for borders
        let start_index = links_scroll;
        let end_index = (start_index + visible_height).min(links.len());
        
        let visible_links = &links[start_index..end_index];
        let items: Vec<ListItem> = visible_links.iter().enumerate().map(|(i, link)| {
            let absolute_index = start_index + i;
            let style = if absolute_index == selected_link {
                Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let content = format!("[{}] {}", link.index, link.text);
            let wrapped_content = fill(&content, (area.width as usize).saturating_sub(6));
            
            ListItem::new(wrapped_content).style(style)
        }).collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("🔗 Links"))
            .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan));

        f.render_widget(list, area);
        
        // Add scrollbar if there are more links than can fit
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

    fn render_help_static(f: &mut Frame, area: Rect) {
        let help_text = vec![
            Line::from(vec![
                Span::styled("↑↓", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" Scroll  "),
                Span::styled("Shift+↑↓", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" Select Link  "),
                Span::styled("Enter", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" Follow  "),
                Span::styled("b", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" Back  "),
            ]),
            Line::from(vec![
                Span::styled("g", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" URL  "),
                Span::styled("h", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" History  "),
                Span::styled("r", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" Refresh  "),
                Span::styled("q", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw(" Quit"),
            ]),
        ];

        let help = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::ALL).title("⌨️ Controls"));
        f.render_widget(help, area);
    }

    fn render_history_static(f: &mut Frame, entries: &[HistoryEntry], current_index: Option<usize>) {
        let area = f.size();
        
        let items: Vec<ListItem> = entries.iter().enumerate().map(|(i, entry)| {
            let marker = if Some(i) == current_index { "➤ " } else { "  " };
            let style = if Some(i) == current_index {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let content = format!("{}{} - {}", marker, entry.title, entry.url);
            let wrapped_content = fill(&content, (area.width as usize).saturating_sub(4));
            ListItem::new(wrapped_content).style(style)
        }).collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("📚 History"))
            .style(Style::default().fg(Color::White));

        f.render_widget(list, area);

        // Help text at bottom
        let help_area = Rect {
            x: area.x,
            y: area.y + area.height - 3,
            width: area.width,
            height: 3,
        };
        
        let help = Paragraph::new("Press any key to return...")
            .style(Style::default().fg(Color::Gray))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(Clear, help_area);
        f.render_widget(help, help_area);
    }

    fn render_url_input_static(f: &mut Frame, input: &str) {
        let area = f.size();
        
        // Center the input dialog
        let popup_area = Rect {
            x: area.width / 4,
            y: area.height / 2 - 2,
            width: area.width / 2,
            height: 4,
        };

        f.render_widget(Clear, popup_area);
        
        let input_widget = Paragraph::new(format!("🌐 {}", input))
            .style(Style::default().fg(Color::White))
            .block(Block::default().borders(Borders::ALL).title("Enter URL"));
        f.render_widget(input_widget, popup_area);
    }

    fn render_error_static(f: &mut Frame, message: &str) {
        let area = f.size();
        
        let popup_area = Rect {
            x: area.width / 8,
            y: area.height / 2 - 3,
            width: area.width * 3 / 4,
            height: 6,
        };

        f.render_widget(Clear, popup_area);

        let wrapped_message = fill(message, (popup_area.width as usize).saturating_sub(4));
        let error_widget = Paragraph::new(wrapped_message)
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title("❌ Error"));
        f.render_widget(error_widget, popup_area);
    }

    pub fn get_user_input(&mut self, state: &UIState) -> Result<UserAction> {
        loop {
            if let Event::Key(key) = event::read()? {
                match state {
                    UIState::URLInput { .. } => {
                        match key.code {
                            KeyCode::Esc => return Ok(UserAction::CancelInput),
                            KeyCode::Enter => {
                                if let UIState::URLInput { input } = state {
                                    return Ok(UserAction::ConfirmInput(input.clone()));
                                }
                            }
                            KeyCode::Backspace => return Ok(UserAction::Backspace),
                            KeyCode::Char(c) => return Ok(UserAction::InputChar(c)),
                            _ => continue,
                        }
                    }
                    UIState::History { .. } => {
                        return Ok(UserAction::GoBack); // Any key returns from history
                    }
                    UIState::Error { .. } => {
                        return Ok(UserAction::Refresh); // Any key dismisses error
                    }
                    _ => {
                        match key.code {
                            KeyCode::Char('q') => return Ok(UserAction::Quit),
                            KeyCode::Char('b') => return Ok(UserAction::GoBack),
                            KeyCode::Char('f') => return Ok(UserAction::GoForward),
                            KeyCode::Char('h') => return Ok(UserAction::ShowHistory),
                            KeyCode::Char('g') => return Ok(UserAction::EnterUrl),
                            KeyCode::Char('r') => return Ok(UserAction::Refresh),
                            KeyCode::Up if key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT) => return Ok(UserAction::SelectPrevLink),
                            KeyCode::Down if key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT) => return Ok(UserAction::SelectNextLink),
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
                        }
                    }
                }
            }
        }
    }

    pub fn scroll_up(&mut self) {
        if self.scroll_position > 0 {
            self.scroll_position -= 1;
        }
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
        // Use conservative estimate when exact height isn't available
        self.update_links_scroll_with_height(10);
    }
    
    fn update_links_scroll_with_height(&mut self, visible_height: usize) {
        if visible_height == 0 {
            return;
        }
        
        // Ensure selected link is visible
        if self.selected_link < self.links_scroll {
            self.links_scroll = self.selected_link;
        } else if self.selected_link >= self.links_scroll + visible_height {
            self.links_scroll = self.selected_link + 1 - visible_height;
        }
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