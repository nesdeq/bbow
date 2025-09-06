// UI module - contains all UI implementations and shared types
// This package provides a clean separation between UI logic and business logic

use crate::links::Link;
use anyhow::Result;

// Re-export UI implementations
pub mod default;
pub mod expi;
pub mod jony;
pub mod robocop;

// Shared UI types and traits
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub url: String,
    pub title: String,
}

#[derive(Debug)]
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
    URLInput {
        input: String,
    },
    URLSuggestions {
        original_url: String,
        error_message: String,
        suggestions: Vec<String>,
        selected_index: usize,
    },
    History {
        entries: Vec<HistoryEntry>,
        current_index: Option<usize>,
    },
    Error {
        message: String,
    },
}

#[derive(Debug)]
pub enum UserAction {
    Quit,
    FollowLink(usize),
    FollowSelectedLink,
    GoBack,
    GoForward,
    ShowHistory,
    EnterUrl,
    ConfirmInput(String),
    CancelInput,
    Refresh,
    ScrollUp,
    ScrollDown,
    SelectPrevLink,
    SelectNextLink,
    InputChar(char),
    Backspace,
    SelectPrevSuggestion,
    SelectNextSuggestion,
    ConfirmSuggestion,
    DismissError,
}

/// Trait that all UI implementations must implement
/// This provides a clean interface between the browser logic and UI rendering
pub trait UIInterface {
    fn new() -> Result<Self>
    where
        Self: Sized;

    fn cleanup(&mut self) -> Result<()>;
    fn render(&mut self, state: &BrowserState) -> Result<()>;
    fn get_user_input(&mut self, state: &BrowserState) -> Result<UserAction>;

    // Scroll management
    fn scroll_up(&mut self);
    fn scroll_down(&mut self);
    fn reset_scroll(&mut self);

    // Link selection
    fn select_prev_link(&mut self, total_links: usize);
    fn select_next_link(&mut self, total_links: usize);
    fn get_selected_link(&self) -> usize;
}
