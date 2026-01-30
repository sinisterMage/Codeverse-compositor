use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use xkbcommon::xkb::Keysym;

#[derive(Debug, Error)]
pub enum KeybindingError {
    #[error("Invalid keybinding format: {0}")]
    InvalidFormat(String),
    #[error("Unknown modifier: {0}")]
    UnknownModifier(String),
    #[error("Unknown key: {0}")]
    UnknownKey(String),
}

/// Modifier keys
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Modifier {
    /// Super/Logo/Windows key
    Super,
    /// Shift key
    Shift,
    /// Control key
    Ctrl,
    /// Alt key
    Alt,
}

/// Parsed keybinding
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Keybinding {
    pub modifiers: Vec<Modifier>,
    pub key: Keysym,
}

impl Keybinding {
    /// Parse a keybinding string like "Super+Shift+q" into a Keybinding
    pub fn parse(s: &str) -> Result<Self, KeybindingError> {
        let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();

        if parts.is_empty() {
            return Err(KeybindingError::InvalidFormat(
                "Empty keybinding string".to_string(),
            ));
        }

        let mut modifiers = Vec::new();

        // All but the last part are modifiers
        for part in &parts[..parts.len() - 1] {
            let modifier = match part.to_lowercase().as_str() {
                "super" | "mod" | "logo" | "win" => Modifier::Super,
                "shift" => Modifier::Shift,
                "ctrl" | "control" => Modifier::Ctrl,
                "alt" => Modifier::Alt,
                _ => return Err(KeybindingError::UnknownModifier(part.to_string())),
            };
            modifiers.push(modifier);
        }

        // Last part is the key
        let key_str = parts[parts.len() - 1];
        let key = string_to_keysym(key_str)
            .ok_or_else(|| KeybindingError::UnknownKey(key_str.to_string()))?;

        Ok(Keybinding { modifiers, key })
    }

    /// Check if this keybinding matches the given input
    pub fn matches(&self, key: Keysym, super_pressed: bool, shift_pressed: bool, ctrl_pressed: bool, alt_pressed: bool) -> bool {
        if self.key != key {
            return false;
        }

        let has_super = self.modifiers.contains(&Modifier::Super);
        let has_shift = self.modifiers.contains(&Modifier::Shift);
        let has_ctrl = self.modifiers.contains(&Modifier::Ctrl);
        let has_alt = self.modifiers.contains(&Modifier::Alt);

        has_super == super_pressed
            && has_shift == shift_pressed
            && has_ctrl == ctrl_pressed
            && has_alt == alt_pressed
    }
}

/// Action that a keybinding triggers
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Quit the compositor
    Quit,
    /// Close focused window
    CloseWindow,
    /// Navigate focus in a direction
    NavigateFocus(Direction),
    /// Split focused container
    Split(SplitDirection),
    /// Switch to workspace (1-10)
    SwitchWorkspace(usize),
    /// Move window to workspace (1-10)
    MoveToWorkspace(usize),
    /// Change layout mode
    ChangeLayout(String),
    /// Toggle floating mode
    ToggleFloating,
    /// Open launcher
    ToggleLauncher,
    /// Reload configuration
    ReloadConfig,
    /// Spawn terminal (for testing)
    SpawnTerminal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

/// Keybinding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingsConfig {
    #[serde(default = "default_keybindings")]
    pub bindings: HashMap<String, Action>,
}

impl Default for KeybindingsConfig {
    fn default() -> Self {
        Self {
            bindings: default_keybindings(),
        }
    }
}

impl KeybindingsConfig {
    /// Get parsed keybindings with their actions
    pub fn parse_all(&self) -> HashMap<Keybinding, Action> {
        let mut result = HashMap::new();

        for (key_str, action) in &self.bindings {
            match Keybinding::parse(key_str) {
                Ok(keybinding) => {
                    result.insert(keybinding, action.clone());
                }
                Err(e) => {
                    tracing::warn!("Failed to parse keybinding '{}': {}", key_str, e);
                }
            }
        }

        result
    }
}

/// Convert a string to a Keysym
fn string_to_keysym(s: &str) -> Option<Keysym> {
    // Handle special keys
    let keysym = match s.to_lowercase().as_str() {
        // Letters
        "a" => Keysym::a,
        "b" => Keysym::b,
        "c" => Keysym::c,
        "d" => Keysym::d,
        "e" => Keysym::e,
        "f" => Keysym::f,
        "g" => Keysym::g,
        "h" => Keysym::h,
        "i" => Keysym::i,
        "j" => Keysym::j,
        "k" => Keysym::k,
        "l" => Keysym::l,
        "m" => Keysym::m,
        "n" => Keysym::n,
        "o" => Keysym::o,
        "p" => Keysym::p,
        "q" => Keysym::q,
        "r" => Keysym::r,
        "s" => Keysym::s,
        "t" => Keysym::t,
        "u" => Keysym::u,
        "v" => Keysym::v,
        "w" => Keysym::w,
        "x" => Keysym::x,
        "y" => Keysym::y,
        "z" => Keysym::z,

        // Numbers
        "0" => Keysym::_0,
        "1" => Keysym::_1,
        "2" => Keysym::_2,
        "3" => Keysym::_3,
        "4" => Keysym::_4,
        "5" => Keysym::_5,
        "6" => Keysym::_6,
        "7" => Keysym::_7,
        "8" => Keysym::_8,
        "9" => Keysym::_9,

        // Function keys
        "f1" => Keysym::F1,
        "f2" => Keysym::F2,
        "f3" => Keysym::F3,
        "f4" => Keysym::F4,
        "f5" => Keysym::F5,
        "f6" => Keysym::F6,
        "f7" => Keysym::F7,
        "f8" => Keysym::F8,
        "f9" => Keysym::F9,
        "f10" => Keysym::F10,
        "f11" => Keysym::F11,
        "f12" => Keysym::F12,

        // Special keys
        "space" => Keysym::space,
        "return" | "enter" => Keysym::Return,
        "escape" | "esc" => Keysym::Escape,
        "tab" => Keysym::Tab,
        "backspace" => Keysym::BackSpace,
        "delete" | "del" => Keysym::Delete,

        // Arrow keys
        "left" => Keysym::Left,
        "right" => Keysym::Right,
        "up" => Keysym::Up,
        "down" => Keysym::Down,

        _ => return None,
    };

    Some(keysym)
}

/// Default keybindings
fn default_keybindings() -> HashMap<String, Action> {
    let mut bindings = HashMap::new();

    // Core
    bindings.insert("Super+Shift+q".to_string(), Action::Quit);
    bindings.insert("Super+Shift+c".to_string(), Action::CloseWindow);
    bindings.insert("Super+Shift+r".to_string(), Action::ReloadConfig);

    // Navigation
    bindings.insert("Super+h".to_string(), Action::NavigateFocus(Direction::Left));
    bindings.insert("Super+j".to_string(), Action::NavigateFocus(Direction::Down));
    bindings.insert("Super+k".to_string(), Action::NavigateFocus(Direction::Up));
    bindings.insert("Super+l".to_string(), Action::NavigateFocus(Direction::Right));

    // Splitting
    bindings.insert("Super+b".to_string(), Action::Split(SplitDirection::Horizontal));
    bindings.insert("Super+v".to_string(), Action::Split(SplitDirection::Vertical));

    // Workspaces
    for i in 1..=9 {
        bindings.insert(format!("Super+{}", i), Action::SwitchWorkspace(i));
        bindings.insert(format!("Super+Shift+{}", i), Action::MoveToWorkspace(i));
    }
    bindings.insert("Super+0".to_string(), Action::SwitchWorkspace(10));
    bindings.insert("Super+Shift+0".to_string(), Action::MoveToWorkspace(10));

    // Layouts
    bindings.insert("Super+e".to_string(), Action::ChangeLayout("splith".to_string()));
    bindings.insert("Super+w".to_string(), Action::ChangeLayout("splitv".to_string()));
    bindings.insert("Super+s".to_string(), Action::ChangeLayout("stacking".to_string()));
    bindings.insert("Super+t".to_string(), Action::ChangeLayout("tabbed".to_string()));

    // Floating
    bindings.insert("Super+Shift+space".to_string(), Action::ToggleFloating);

    // Launcher
    bindings.insert("Super+d".to_string(), Action::ToggleLauncher);

    // Testing
    bindings.insert("F12".to_string(), Action::SpawnTerminal);

    bindings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_keybinding() {
        let kb = Keybinding::parse("Super+d").unwrap();
        assert_eq!(kb.modifiers, vec![Modifier::Super]);
        assert_eq!(kb.key, Keysym::d);
    }

    #[test]
    fn test_parse_multiple_modifiers() {
        let kb = Keybinding::parse("Super+Shift+q").unwrap();
        assert_eq!(kb.modifiers, vec![Modifier::Super, Modifier::Shift]);
        assert_eq!(kb.key, Keysym::q);
    }

    #[test]
    fn test_parse_function_key() {
        let kb = Keybinding::parse("F12").unwrap();
        assert_eq!(kb.modifiers, vec![]);
        assert_eq!(kb.key, Keysym::F12);
    }

    #[test]
    fn test_parse_with_ctrl() {
        let kb = Keybinding::parse("Ctrl+c").unwrap();
        assert_eq!(kb.modifiers, vec![Modifier::Ctrl]);
        assert_eq!(kb.key, Keysym::c);
    }

    #[test]
    fn test_parse_invalid_modifier() {
        let result = Keybinding::parse("Invalid+d");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_key() {
        let result = Keybinding::parse("Super+invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_keybinding_matches() {
        let kb = Keybinding::parse("Super+Shift+q").unwrap();
        assert!(kb.matches(Keysym::q, true, true, false, false));
        assert!(!kb.matches(Keysym::q, true, false, false, false));
        assert!(!kb.matches(Keysym::q, false, true, false, false));
        assert!(!kb.matches(Keysym::d, true, true, false, false));
    }

    #[test]
    fn test_default_keybindings() {
        let config = KeybindingsConfig::default();
        assert!(!config.bindings.is_empty());
        assert!(config.bindings.contains_key("Super+d"));
        assert_eq!(config.bindings.get("Super+d"), Some(&Action::ToggleLauncher));
    }

    #[test]
    fn test_parse_all_default_bindings() {
        let config = KeybindingsConfig::default();
        let parsed = config.parse_all();

        // Should successfully parse all default bindings
        assert!(!parsed.is_empty());

        // Check a specific binding
        let launcher_kb = Keybinding::parse("Super+d").unwrap();
        assert_eq!(parsed.get(&launcher_kb), Some(&Action::ToggleLauncher));
    }
}
