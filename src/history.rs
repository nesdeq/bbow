use std::collections::VecDeque;

const MAX_HISTORY_SIZE: usize = 100;

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub url: String,
    pub title: String,
}

pub struct History {
    entries: VecDeque<HistoryEntry>,
    current_index: Option<usize>,
}

impl History {
    pub fn new() -> Self {
        Self {
            entries: VecDeque::with_capacity(MAX_HISTORY_SIZE),
            current_index: None,
        }
    }

    pub fn add(&mut self, url: String, title: String) {
        if let Some(current) = self.current_index {
            self.entries.truncate(current + 1);
        }

        self.entries.push_back(HistoryEntry { url, title });
        self.current_index = Some(self.entries.len() - 1);

        if self.entries.len() > MAX_HISTORY_SIZE {
            self.entries.pop_front();
            if let Some(ref mut current) = self.current_index {
                *current = current.saturating_sub(1);
            }
        }
    }

    pub fn can_go_back(&self) -> bool {
        self.current_index.is_some_and(|i| i > 0)
    }

    pub fn can_go_forward(&self) -> bool {
        self.current_index
            .is_some_and(|i| i < self.entries.len() - 1)
    }

    pub fn go_back(&mut self) -> Option<&HistoryEntry> {
        if self.can_go_back() {
            self.current_index = self.current_index.map(|i| i - 1);
            self.current()
        } else {
            None
        }
    }

    pub fn go_forward(&mut self) -> Option<&HistoryEntry> {
        if self.can_go_forward() {
            self.current_index = self.current_index.map(|i| i + 1);
            self.current()
        } else {
            None
        }
    }

    pub fn current(&self) -> Option<&HistoryEntry> {
        self.current_index.and_then(|i| self.entries.get(i))
    }

    pub fn list(&self) -> Vec<&HistoryEntry> {
        self.entries.iter().collect()
    }
}
