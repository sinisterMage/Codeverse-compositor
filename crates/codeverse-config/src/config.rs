use crate::keybindings::KeybindingsConfig;
use crate::theme::NordTheme;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// Main configuration struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,

    #[serde(default)]
    pub theme: ThemeConfig,

    #[serde(default)]
    pub keybindings: KeybindingsConfig,

    #[serde(default)]
    pub workspaces: WorkspacesConfig,

    #[serde(default)]
    pub launcher: LauncherConfig,
}

/// General compositor settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Border width in pixels
    #[serde(default = "default_border_width")]
    pub border_width: u32,

    /// Gap width between windows in pixels
    #[serde(default = "default_gap_width")]
    pub gap_width: u32,

    /// Default layout mode for new workspaces
    #[serde(default = "default_layout")]
    pub default_layout: String,

    /// Focus follows mouse
    #[serde(default)]
    pub focus_follows_mouse: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            border_width: default_border_width(),
            gap_width: default_gap_width(),
            default_layout: default_layout(),
            focus_follows_mouse: false,
        }
    }
}

fn default_border_width() -> u32 {
    2
}

fn default_gap_width() -> u32 {
    10
}

fn default_layout() -> String {
    "splith".to_string()
}

/// Theme configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    /// Use Nord theme (currently the only option)
    #[serde(default = "default_use_nord")]
    pub use_nord: bool,

    /// Custom focused border color (hex format: #RRGGBB)
    pub focused_border: Option<String>,

    /// Custom unfocused border color (hex format: #RRGGBB)
    pub unfocused_border: Option<String>,

    /// Custom background color (hex format: #RRGGBB)
    pub background: Option<String>,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            use_nord: default_use_nord(),
            focused_border: None,
            unfocused_border: None,
            background: None,
        }
    }
}

fn default_use_nord() -> bool {
    true
}

/// Workspaces configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspacesConfig {
    /// Number of workspaces (1-10)
    #[serde(default = "default_workspace_count")]
    pub count: usize,

    /// Workspace names
    #[serde(default)]
    pub names: Vec<String>,
}

impl Default for WorkspacesConfig {
    fn default() -> Self {
        Self {
            count: default_workspace_count(),
            names: vec![],
        }
    }
}

fn default_workspace_count() -> usize {
    10
}

/// Launcher configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherConfig {
    /// Additional paths to search for .desktop files
    #[serde(default)]
    pub additional_paths: Vec<String>,

    /// Maximum number of results to show
    #[serde(default = "default_max_results")]
    pub max_results: usize,

    /// Show descriptions in launcher
    #[serde(default = "default_show_descriptions")]
    pub show_descriptions: bool,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            additional_paths: vec![],
            max_results: default_max_results(),
            show_descriptions: default_show_descriptions(),
        }
    }
}

fn default_max_results() -> usize {
    10
}

fn default_show_descriptions() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            theme: ThemeConfig::default(),
            keybindings: KeybindingsConfig::default(),
            workspaces: WorkspacesConfig::default(),
            launcher: LauncherConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from the default location
    /// (~/.config/codeverse-compositor/config.toml)
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if !path.exists() {
            info!(
                "Config file not found at {:?}, using defaults",
                path
            );
            return Ok(Self::default());
        }

        Self::load_from_path(&path)
    }

    /// Load configuration from a specific path
    pub fn load_from_path(path: &PathBuf) -> Result<Self> {
        debug!("Loading config from {:?}", path);

        let contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {:?}", path))?;

        let config: Config = toml::from_str(&contents)
            .with_context(|| format!("Failed to parse config file: {:?}", path))?;

        info!("Successfully loaded config from {:?}", path);
        Ok(config)
    }

    /// Save configuration to the default location
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {:?}", parent))?;
        }

        let contents = toml::to_string_pretty(self)
            .context("Failed to serialize config to TOML")?;

        fs::write(&path, contents)
            .with_context(|| format!("Failed to write config file: {:?}", path))?;

        info!("Successfully saved config to {:?}", path);
        Ok(())
    }

    /// Get the default config file path
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Could not determine config directory")?;

        Ok(config_dir.join("codeverse-compositor").join("config.toml"))
    }

    /// Create a default config file if it doesn't exist
    pub fn create_default_if_missing() -> Result<()> {
        let path = Self::config_path()?;

        if path.exists() {
            debug!("Config file already exists at {:?}", path);
            return Ok(());
        }

        info!("Creating default config file at {:?}", path);
        let config = Self::default();
        config.save()?;

        Ok(())
    }

    /// Get the theme based on configuration
    pub fn get_theme(&self) -> NordTheme {
        // For now, always use Nord theme
        // In the future, we can support custom themes here
        NordTheme::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.general.border_width, 2);
        assert_eq!(config.general.gap_width, 10);
        assert_eq!(config.workspaces.count, 10);
        assert!(config.theme.use_nord);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("[general]"));
        assert!(toml_str.contains("[workspaces]"));
        // Keybindings should be serialized somehow (might be nested)
        assert!(!toml_str.is_empty());
    }

    #[test]
    fn test_config_deserialization() {
        let toml_str = r#"
[general]
border_width = 3
gap_width = 15

[workspaces]
count = 5

[launcher]
max_results = 20
"#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.general.border_width, 3);
        assert_eq!(config.general.gap_width, 15);
        assert_eq!(config.workspaces.count, 5);
        assert_eq!(config.launcher.max_results, 20);
    }

    #[test]
    fn test_partial_config() {
        // Test that partial config with missing sections uses defaults
        let toml_str = r#"
[general]
border_width = 4
"#;

        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.general.border_width, 4);
        assert_eq!(config.general.gap_width, 10); // Default
        assert_eq!(config.workspaces.count, 10); // Default
    }
}
