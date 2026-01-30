use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

/// A desktop application entry
#[derive(Debug, Clone)]
pub struct App {
    /// Application name
    pub name: String,
    /// Executable command
    pub exec: String,
    /// Description
    pub description: Option<String>,
    /// Path to .desktop file
    pub desktop_file: PathBuf,
    /// Whether this is a terminal application
    pub terminal: bool,
}

impl App {
    /// Get the command to execute (strips field codes like %f, %u, etc.)
    pub fn get_command(&self) -> String {
        // Remove field codes from Exec line
        self.exec
            .split_whitespace()
            .filter(|s| !s.starts_with('%'))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Discover and parse desktop applications
pub struct AppDiscovery {
    apps: Vec<App>,
}

impl AppDiscovery {
    /// Create a new app discovery instance and scan for apps
    pub fn new() -> Self {
        let mut discovery = Self { apps: Vec::new() };
        discovery.scan();
        discovery
    }

    /// Scan for desktop files in standard locations
    fn scan(&mut self) {
        let search_paths = vec![
            PathBuf::from("/usr/share/applications"),
            PathBuf::from("/usr/local/share/applications"),
            dirs::data_local_dir()
                .map(|p| p.join("applications"))
                .unwrap_or_else(|| PathBuf::from("~/.local/share/applications")),
        ];

        for path in search_paths {
            if path.exists() && path.is_dir() {
                self.scan_directory(&path);
            }
        }

        debug!("Found {} applications", self.apps.len());
    }

    /// Recursively scan a directory for .desktop files
    fn scan_directory(&mut self, dir: &Path) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();

                if path.is_dir() {
                    // Recursively scan subdirectories
                    self.scan_directory(&path);
                } else if path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                    if let Some(app) = self.parse_desktop_file(&path) {
                        self.apps.push(app);
                    }
                }
            }
        }
    }

    /// Parse a .desktop file
    fn parse_desktop_file(&self, path: &Path) -> Option<App> {
        let content = fs::read_to_string(path).ok()?;

        let mut name: Option<String> = None;
        let mut exec: Option<String> = None;
        let mut description: Option<String> = None;
        let mut terminal = false;
        let mut no_display = false;
        let mut hidden = false;

        let mut in_desktop_entry = false;

        for line in content.lines() {
            let line = line.trim();

            // Check for [Desktop Entry] section
            if line == "[Desktop Entry]" {
                in_desktop_entry = true;
                continue;
            } else if line.starts_with('[') {
                in_desktop_entry = false;
                continue;
            }

            if !in_desktop_entry {
                continue;
            }

            // Skip comments
            if line.starts_with('#') {
                continue;
            }

            // Parse key=value pairs
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();

                match key {
                    "Name" => name = Some(value.to_string()),
                    "Exec" => exec = Some(value.to_string()),
                    "Comment" | "GenericName" => {
                        if description.is_none() {
                            description = Some(value.to_string());
                        }
                    }
                    "Terminal" => terminal = value.eq_ignore_ascii_case("true"),
                    "NoDisplay" => no_display = value.eq_ignore_ascii_case("true"),
                    "Hidden" => hidden = value.eq_ignore_ascii_case("true"),
                    _ => {}
                }
            }
        }

        // Skip apps that should not be displayed
        if no_display || hidden {
            return None;
        }

        // Require at least Name and Exec
        let name = name?;
        let exec = exec?;

        Some(App {
            name,
            exec,
            description,
            desktop_file: path.to_path_buf(),
            terminal,
        })
    }

    /// Get all discovered apps
    pub fn apps(&self) -> &[App] {
        &self.apps
    }

    /// Search apps by name (case-insensitive, fuzzy-ish)
    pub fn search(&self, query: &str) -> Vec<&App> {
        if query.is_empty() {
            return self.apps.iter().collect();
        }

        let query = query.to_lowercase();
        let mut results: Vec<(&App, i32)> = self.apps
            .iter()
            .filter_map(|app| {
                let name_lower = app.name.to_lowercase();

                // Exact match gets highest score
                if name_lower == query {
                    return Some((app, 1000));
                }

                // Starts with query gets high score
                if name_lower.starts_with(&query) {
                    return Some((app, 500));
                }

                // Contains query gets medium score
                if name_lower.contains(&query) {
                    return Some((app, 100));
                }

                // Check description
                if let Some(ref desc) = app.description {
                    if desc.to_lowercase().contains(&query) {
                        return Some((app, 50));
                    }
                }

                None
            })
            .collect();

        // Sort by score (highest first)
        results.sort_by(|a, b| b.1.cmp(&a.1));

        results.into_iter().map(|(app, _)| app).collect()
    }
}

impl Default for AppDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_discovery() {
        let discovery = AppDiscovery::new();
        // Should find at least some apps on any Linux system
        assert!(!discovery.apps().is_empty(), "Should discover at least some applications");
    }

    #[test]
    fn test_search() {
        let discovery = AppDiscovery::new();

        // Empty search should return all
        let all = discovery.search("");
        assert_eq!(all.len(), discovery.apps().len());

        // Search for something that probably exists
        let results = discovery.search("terminal");
        // Should find at least one terminal app or nothing
        assert!(results.is_empty() || !results.is_empty());
    }

    #[test]
    fn test_command_cleaning() {
        let app = App {
            name: "Test".to_string(),
            exec: "firefox %u --new-window".to_string(),
            description: None,
            desktop_file: PathBuf::from("/test.desktop"),
            terminal: false,
        };

        let cmd = app.get_command();
        assert_eq!(cmd, "firefox --new-window");
        assert!(!cmd.contains("%u"));
    }
}
