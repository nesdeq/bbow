// RoboCop-inspired UI for BBOW
// Capturing the 1987 cyberpunk aesthetic: corporate chrome, digital amber displays,
// and the cold efficiency of OCP's dystopian future

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

pub struct RobocopUI {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    scroll_position: u16,
    selected_link: usize,
    links_scroll: usize,
    max_scroll: u16,
}

// RoboCop 1987 color palette - Corporate dystopian future
const PRIMARY_AMBER: Color = Color::Rgb(255, 191, 0);     // Classic amber terminal display
const CHROME_BLUE: Color = Color::Rgb(102, 178, 255);    // Cold corporate chrome blue
const WARNING_RED: Color = Color::Rgb(255, 89, 94);      // Alert/danger red
const SYSTEM_GREEN: Color = Color::Rgb(0, 255, 127);     // Matrix-style green
const STEEL_GRAY: Color = Color::Rgb(169, 169, 169);     // Metallic interface elements
const DARK_CHROME: Color = Color::Rgb(47, 79, 79);       // Dark steel backgrounds
const CONSOLE_BLACK: Color = Color::Rgb(20, 20, 20);     // Deep system black
const DATA_WHITE: Color = Color::Rgb(240, 248, 255);     // Clean data display

impl UIInterface for RobocopUI {
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
                    self.terminal.size()?.height.saturating_sub(12) as usize,
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

impl RobocopUI {
    fn render_loading(f: &mut Frame, url: &str, progress: u16, stage: &str) {
        let area = f.size();

        // Corporate-style header bar
        let header_area = Rect {
            x: 0,
            y: 0,
            width: area.width,
            height: 1,
        };

        f.render_widget(
            Paragraph::new("═══════════ OMNI CONSUMER PRODUCTS NETWORK INTERFACE ═══════════")
                .style(Style::default().fg(CHROME_BLUE).bg(CONSOLE_BLACK))
                .alignment(Alignment::Center),
            header_area,
        );

        // Main loading interface
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints([
                Constraint::Length(1),  // Spacer
                Constraint::Length(3),  // System status
                Constraint::Length(1),  // Spacer
                Constraint::Length(3),  // URL display
                Constraint::Length(1),  // Spacer
                Constraint::Length(3),  // Progress bar
                Constraint::Length(1),  // Spacer
                Constraint::Length(2),  // Current operation
                Constraint::Min(0),     // Bottom spacer
            ])
            .split(Rect {
                x: 0,
                y: 1,
                width: area.width,
                height: area.height - 1,
            });

        // System status
        f.render_widget(
            Paragraph::new("[ SYSTEM STATUS: ONLINE ]")
                .style(Style::default().fg(SYSTEM_GREEN).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(DARK_CHROME))
                ),
            main_chunks[1],
        );

        // URL display with terminal styling
        f.render_widget(
            Paragraph::new(format!("TARGET: {}", url))
                .style(Style::default().fg(PRIMARY_AMBER))
                .wrap(Wrap { trim: true })
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(DARK_CHROME))
                        .title("NETWORK DESTINATION")
                        .title_style(Style::default().fg(STEEL_GRAY)),
                ),
            main_chunks[3],
        );

        // Corporate progress bar - industrial design
        f.render_widget(
            Gauge::default()
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(DARK_CHROME))
                        .title("DATA ACQUISITION")
                        .title_style(Style::default().fg(STEEL_GRAY)),
                )
                .gauge_style(Style::default().fg(CHROME_BLUE).bg(CONSOLE_BLACK))
                .percent(progress)
                .label(format!("{}% COMPLETE", progress))
                .use_unicode(true),
            main_chunks[5],
        );

        // Current operation in monospace corporate style
        f.render_widget(
            Paragraph::new(format!("OPERATION: {}", stage.to_uppercase()))
                .style(Style::default().fg(DATA_WHITE))
                .alignment(Alignment::Center),
            main_chunks[7],
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

        // Corporate header bar
        let header_area = Rect {
            x: 0,
            y: 0,
            width: area.width,
            height: 1,
        };

        f.render_widget(
            Paragraph::new("═══════════ OCP NETWORK TERMINAL ═══════════")
                .style(Style::default().fg(CHROME_BLUE).bg(CONSOLE_BLACK))
                .alignment(Alignment::Center),
            header_area,
        );

        // Main interface layout
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),  // Spacer
                Constraint::Length(6),  // Header info (increased for bordered content)
                Constraint::Min(10),    // Content area
                Constraint::Length(3),  // Status bar
            ])
            .split(Rect {
                x: 0,
                y: 1,
                width: area.width,
                height: area.height - 1,
            });

        Self::render_header(f, main_chunks[1], url, title);

        // Content layout - corporate split screen
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(main_chunks[2]);

        Self::render_summary(f, content_chunks[0], summary, scroll_pos);
        Self::render_links(f, content_chunks[1], links, selected_link, links_scroll);
        Self::render_status_bar(f, main_chunks[3]);
    }

    fn render_header(f: &mut Frame, area: Rect, url: &str, title: &str) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Length(3)])
            .split(area);

        // Title in corporate amber display style
        f.render_widget(
            Paragraph::new(title)
                .style(Style::default().fg(PRIMARY_AMBER).add_modifier(Modifier::BOLD))
                .wrap(Wrap { trim: true })
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(DARK_CHROME))
                        .title("DOCUMENT TITLE")
                        .title_style(Style::default().fg(STEEL_GRAY)),
                ),
            chunks[0],
        );

        // URL in system green
        f.render_widget(
            Paragraph::new(url)
                .style(Style::default().fg(SYSTEM_GREEN))
                .wrap(Wrap { trim: true })
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(DARK_CHROME))
                        .title("NETWORK ADDRESS")
                        .title_style(Style::default().fg(STEEL_GRAY)),
                ),
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

        if visible_lines.is_empty() {
            f.render_widget(
                Paragraph::new("[ NO DATA AVAILABLE ]")
                    .style(Style::default().fg(STEEL_GRAY))
                    .alignment(Alignment::Center)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(DARK_CHROME))
                            .title("DATA ANALYSIS")
                            .title_style(Style::default().fg(STEEL_GRAY)),
                    ),
                area,
            );
            return;
        }

        // Main data display with corporate styling
        f.render_widget(
            Paragraph::new(visible_lines)
                .style(Style::default().fg(DATA_WHITE))
                .wrap(Wrap { trim: false })
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(DARK_CHROME))
                        .title("DATA ANALYSIS")
                        .title_style(Style::default().fg(STEEL_GRAY)),
                ),
            area,
        );

        // Corporate-style scroll indicator
        let max_scroll = ui_common::calculate_max_scroll_for_markdown(
            summary,
            width,
            visible_height,
            Self::style_markdown_element,
        );

        if max_scroll > 0 && area.width > 2 && area.height > 2 {
            let scroll_pos_ratio = (scroll_pos as f32 / max_scroll as f32).min(1.0);
            let indicator_y_offset =
                (scroll_pos_ratio * (area.height.saturating_sub(4)) as f32) as u16;
            let indicator_y = area.y + 2 + indicator_y_offset;

            if indicator_y < area.y + area.height - 1 && area.x + area.width > 2 {
                f.render_widget(
                    Paragraph::new("█").style(Style::default().fg(CHROME_BLUE)),
                    Rect {
                        x: area.x + area.width - 2,
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
            MarkdownElement::Header1(_) => Style::default()
                .fg(PRIMARY_AMBER)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            MarkdownElement::Header2(_) => Style::default()
                .fg(PRIMARY_AMBER)
                .add_modifier(Modifier::BOLD),
            MarkdownElement::Header3(_) => Style::default()
                .fg(CHROME_BLUE)
                .add_modifier(Modifier::BOLD),
            MarkdownElement::Header4(_) => Style::default()
                .fg(STEEL_GRAY)
                .add_modifier(Modifier::BOLD),
            MarkdownElement::Bold(_) => Style::default()
                .fg(DATA_WHITE)
                .add_modifier(Modifier::BOLD),
            MarkdownElement::Italic(_) => Style::default()
                .fg(SYSTEM_GREEN)
                .add_modifier(Modifier::ITALIC),
            MarkdownElement::Code(_) => Style::default()
                .fg(PRIMARY_AMBER)
                .bg(CONSOLE_BLACK),
            MarkdownElement::Normal(_) => Style::default().fg(DATA_WHITE),
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
                Paragraph::new("[ NO LINKS DETECTED ]")
                    .style(Style::default().fg(STEEL_GRAY))
                    .alignment(Alignment::Center)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(DARK_CHROME))
                            .title("NAVIGATION LINKS")
                            .title_style(Style::default().fg(STEEL_GRAY)),
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

                let marker = if is_selected { "►" } else { " " };
                let content = format!("{} [{}] {}", marker, link.index, link.text);

                let style = if is_selected {
                    Style::default()
                        .fg(CONSOLE_BLACK)
                        .bg(PRIMARY_AMBER)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(SYSTEM_GREEN)
                };

                let wrapped_content = fill(&content, area.width.saturating_sub(6) as usize);
                ListItem::new(wrapped_content).style(style)
            })
            .collect();

        f.render_widget(
            List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(DARK_CHROME))
                        .title("NAVIGATION LINKS")
                        .title_style(Style::default().fg(STEEL_GRAY)),
                )
                .highlight_style(
                    Style::default()
                        .fg(CONSOLE_BLACK)
                        .bg(PRIMARY_AMBER)
                        .add_modifier(Modifier::BOLD),
                ),
            area,
        );
    }

    fn render_status_bar(f: &mut Frame, area: Rect) {
        // Corporate command interface
        let command_line = vec![
            Line::from(vec![
                Span::styled("COMMANDS: ", Style::default().fg(STEEL_GRAY)),
                Span::styled("↑↓", Style::default().fg(PRIMARY_AMBER).add_modifier(Modifier::BOLD)),
                Span::styled(" SCROLL  ", Style::default().fg(DATA_WHITE)),
                Span::styled("⏎", Style::default().fg(PRIMARY_AMBER).add_modifier(Modifier::BOLD)),
                Span::styled(" EXECUTE  ", Style::default().fg(DATA_WHITE)),
                Span::styled("G", Style::default().fg(PRIMARY_AMBER).add_modifier(Modifier::BOLD)),
                Span::styled(" URL  ", Style::default().fg(DATA_WHITE)),
                Span::styled("Q", Style::default().fg(WARNING_RED).add_modifier(Modifier::BOLD)),
                Span::styled(" TERMINATE", Style::default().fg(DATA_WHITE)),
            ]),
        ];

        f.render_widget(
            Paragraph::new(command_line)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(DARK_CHROME))
                        .title("SYSTEM COMMANDS")
                        .title_style(Style::default().fg(STEEL_GRAY)),
                ),
            area,
        );
    }

    fn render_history(f: &mut Frame, entries: &[HistoryEntry], current_index: Option<usize>) {
        let area = f.size();

        // Corporate header
        let header_area = Rect {
            x: 0,
            y: 0,
            width: area.width,
            height: 1,
        };

        f.render_widget(
            Paragraph::new("═══════════ ACCESS HISTORY LOG ═══════════")
                .style(Style::default().fg(CHROME_BLUE).bg(CONSOLE_BLACK))
                .alignment(Alignment::Center),
            header_area,
        );

        let content_area = Rect {
            x: 0,
            y: 1,
            width: area.width,
            height: area.height - 3,
        };

        let items: Vec<ListItem> = entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let is_current = Some(i) == current_index;
                let marker = if is_current { "►" } else { " " };
                let style = if is_current {
                    Style::default().fg(PRIMARY_AMBER).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(DATA_WHITE)
                };

                let content = format!("{} {} - {}", marker, entry.title, entry.url);
                let wrapped_content = fill(&content, content_area.width.saturating_sub(4) as usize);
                ListItem::new(wrapped_content).style(style)
            })
            .collect();

        f.render_widget(
            List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(DARK_CHROME))
                        .title("NETWORK HISTORY")
                        .title_style(Style::default().fg(STEEL_GRAY)),
                )
                .style(Style::default().fg(DATA_WHITE)),
            content_area,
        );

        let footer_area = Rect {
            x: 0,
            y: area.height - 2,
            width: area.width,
            height: 2,
        };

        f.render_widget(
            Paragraph::new("PRESS ANY KEY TO RETURN TO MAIN INTERFACE")
                .style(Style::default().fg(SYSTEM_GREEN))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(DARK_CHROME)),
                ),
            footer_area,
        );
    }

    fn render_url_input(f: &mut Frame, input: &str) {
        let area = f.size();
        let popup_area = Rect {
            x: area.width / 6,
            y: area.height / 2 - 3,
            width: area.width * 2 / 3,
            height: 6,
        };

        f.render_widget(Clear, popup_area);

        f.render_widget(
            Paragraph::new(format!("ENTER NETWORK ADDRESS:\n\n{}", input))
                .style(Style::default().fg(DATA_WHITE))
                .alignment(Alignment::Left)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(PRIMARY_AMBER))
                        .title("NETWORK INPUT")
                        .title_style(Style::default().fg(PRIMARY_AMBER)),
                ),
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
            x: area.width / 10,
            y: area.height / 6,
            width: area.width * 4 / 5,
            height: area.height * 2 / 3,
        };

        f.render_widget(Clear, popup_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Error
                Constraint::Length(3), // Original URL
                Constraint::Min(5),    // Suggestions
                Constraint::Length(2), // Commands
            ])
            .split(popup_area.inner(&Margin {
                horizontal: 1,
                vertical: 1,
            }));

        // Error display
        f.render_widget(
            Paragraph::new(format!("CONNECTION FAILED: {}", error_message))
                .style(Style::default().fg(WARNING_RED).add_modifier(Modifier::BOLD))
                .wrap(Wrap { trim: true })
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(WARNING_RED))
                        .title("SYSTEM ERROR")
                        .title_style(Style::default().fg(WARNING_RED)),
                ),
            chunks[0],
        );

        // Original URL
        f.render_widget(
            Paragraph::new(format!("ORIGINAL TARGET: {}", original_url))
                .style(Style::default().fg(SYSTEM_GREEN))
                .wrap(Wrap { trim: true })
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(DARK_CHROME))
                        .title("INPUT")
                        .title_style(Style::default().fg(STEEL_GRAY)),
                ),
            chunks[1],
        );

        // Suggestions
        let suggestion_items: Vec<ListItem> = suggestions
            .iter()
            .enumerate()
            .map(|(i, suggestion)| {
                let is_selected = i == selected_index;
                let marker = if is_selected { "►" } else { " " };
                let style = if is_selected {
                    Style::default()
                        .fg(CONSOLE_BLACK)
                        .bg(PRIMARY_AMBER)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(DATA_WHITE)
                };
                ListItem::new(format!("{} {}", marker, suggestion)).style(style)
            })
            .collect();

        f.render_widget(
            List::new(suggestion_items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(DARK_CHROME))
                        .title("ALTERNATIVE TARGETS")
                        .title_style(Style::default().fg(STEEL_GRAY)),
                )
                .highlight_style(
                    Style::default()
                        .fg(CONSOLE_BLACK)
                        .bg(PRIMARY_AMBER)
                        .add_modifier(Modifier::BOLD),
                ),
            chunks[2],
        );

        f.render_widget(
            Paragraph::new("↑↓ SELECT • ⏎ CONNECT • ESC ABORT • Q TERMINATE")
                .style(Style::default().fg(CHROME_BLUE))
                .alignment(Alignment::Center),
            chunks[3],
        );

        // Border for the entire popup
        f.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(CHROME_BLUE))
                .title("OCP NETWORK ERROR RECOVERY")
                .title_style(Style::default().fg(CHROME_BLUE)),
            popup_area,
        );
    }

    fn render_error(f: &mut Frame, message: &str) {
        let area = f.size();
        let popup_area = Rect {
            x: area.width / 8,
            y: area.height / 2 - 4,
            width: area.width * 3 / 4,
            height: 8,
        };

        f.render_widget(Clear, popup_area);

        f.render_widget(
            Paragraph::new(format!(
                "CRITICAL SYSTEM ERROR\n\n{}\n\nPRESS ANY KEY TO ACKNOWLEDGE",
                message.to_uppercase()
            ))
            .style(
                Style::default()
                    .fg(WARNING_RED)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(WARNING_RED))
                    .title("⚠ SYSTEM ALERT ⚠")
                    .title_style(
                        Style::default()
                            .fg(WARNING_RED)
                            .add_modifier(Modifier::BOLD),
                    ),
            ),
            popup_area,
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
        let content_width = terminal_size.width * 70 / 100; // 70% for content area
        let width = content_width.saturating_sub(4) as usize; // same as area.width.saturating_sub(4)
        let content_height = terminal_size.height.saturating_sub(1 + 4 + 3); // header + info + status
        let visible_height = content_height.saturating_sub(2) as usize; // borders

        self.max_scroll = ui_common::calculate_max_scroll_for_markdown(
            summary,
            width,
            visible_height,
            Self::style_markdown_element,
        );
    }
}