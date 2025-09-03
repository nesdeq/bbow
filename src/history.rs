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
    max_size: usize,
}

impl History {
    pub fn new() -> Self {
        Self {
            entries: VecDeque::new(),
            current_index: None,
            max_size: MAX_HISTORY_SIZE,
        }
    }
    
    pub fn add(&mut self, url: String, title: String) {
        let entry = HistoryEntry { url, title };
        
        // If we're not at the end of history, remove everything after current position
        if let Some(current) = self.current_index {
            let remove_count = self.entries.len() - current - 1;
            for _ in 0..remove_count {
                self.entries.pop_back();
            }
        }
        
        // Add new entry
        self.entries.push_back(entry);
        self.current_index = Some(self.entries.len() - 1);
        
        // Maintain max size
        while self.entries.len() > self.max_size {
            self.entries.pop_front();
            if let Some(ref mut current) = self.current_index {
                if *current > 0 {
                    *current -= 1;
                } else {
                    self.current_index = None;
                }
            }
        }
    }
    
    pub fn can_go_back(&self) -> bool {
        self.current_index.map_or(false, |i| i > 0)
    }
    
    pub fn can_go_forward(&self) -> bool {
        self.current_index.map_or(false, |i| i < self.entries.len() - 1)
    }
    
    pub fn go_back(&mut self) -> Option<&HistoryEntry> {
        if self.can_go_back() {
            if let Some(ref mut current) = self.current_index {
                *current -= 1;
                return self.entries.get(*current);
            }
        }
        None
    }
    
    pub fn go_forward(&mut self) -> Option<&HistoryEntry> {
        if self.can_go_forward() {
            if let Some(ref mut current) = self.current_index {
                *current += 1;
                return self.entries.get(*current);
            }
        }
        None
    }
    
    pub fn current(&self) -> Option<&HistoryEntry> {
        self.current_index.and_then(|i| self.entries.get(i))
    }
    
    pub fn list(&self) -> Vec<&HistoryEntry> {
        self.entries.iter().collect()
    }
    
}