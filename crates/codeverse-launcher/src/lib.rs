pub mod apps;

pub use apps::{App, AppDiscovery};

/// Launcher state for managing application search and selection
pub struct LauncherState {
    /// Application discovery instance
    discovery: AppDiscovery,

    /// Current search query
    query: String,

    /// Filtered and sorted app results
    results: Vec<App>,

    /// Currently selected index in results
    selected_index: usize,
}

impl LauncherState {
    /// Create a new launcher state
    pub fn new() -> Self {
        let discovery = AppDiscovery::new();
        let results = discovery.apps().to_vec();

        Self {
            discovery,
            query: String::new(),
            results,
            selected_index: 0,
        }
    }

    /// Update the search query and refresh results
    pub fn set_query(&mut self, query: String) {
        self.query = query;
        self.results = self.discovery.search(&self.query)
            .into_iter()
            .cloned()
            .collect();
        self.selected_index = 0;
    }

    /// Get the current search query
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Add a character to the search query
    pub fn push_char(&mut self, ch: char) {
        self.query.push(ch);
        self.results = self.discovery.search(&self.query)
            .into_iter()
            .cloned()
            .collect();
        self.selected_index = 0;
    }

    /// Remove last character from search query
    pub fn pop_char(&mut self) {
        self.query.pop();
        self.results = self.discovery.search(&self.query)
            .into_iter()
            .cloned()
            .collect();
        self.selected_index = 0;
    }

    /// Get current search results
    pub fn results(&self) -> &[App] {
        &self.results
    }

    /// Get currently selected app
    pub fn selected_app(&self) -> Option<&App> {
        self.results.get(self.selected_index)
    }

    /// Get selected index
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// Move selection up
    pub fn select_previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        if self.selected_index + 1 < self.results.len() {
            self.selected_index += 1;
        }
    }

    /// Reset launcher state
    pub fn reset(&mut self) {
        self.query.clear();
        self.results = self.discovery.apps().to_vec();
        self.selected_index = 0;
    }
}

impl Default for LauncherState {
    fn default() -> Self {
        Self::new()
    }
}
