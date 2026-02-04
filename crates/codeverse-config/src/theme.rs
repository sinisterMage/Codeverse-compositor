/// Nord color palette
/// Based on https://www.nordtheme.com/

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NordColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl NordColor {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Convert to f32 array for rendering (0.0-1.0 range)
    pub fn to_f32_array(&self) -> [f32; 4] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        ]
    }

    /// Convert to RGB tuple (for integer-based APIs)
    pub fn to_rgb(&self) -> (u8, u8, u8) {
        (self.r, self.g, self.b)
    }

    /// Convert to RGBA tuple
    pub fn to_rgba(&self) -> (u8, u8, u8, u8) {
        (self.r, self.g, self.b, self.a)
    }
}

/// Complete Nord color palette
#[derive(Debug, Clone, Copy)]
pub struct NordColors {
    // Polar Night (dark backgrounds)
    pub nord0: NordColor,  // #2e3440 - Darkest background
    pub nord1: NordColor,  // #3b4252 - Dark background
    pub nord2: NordColor,  // #434c5e - Medium dark
    pub nord3: NordColor,  // #4c566a - Light dark

    // Snow Storm (light foregrounds)
    pub nord4: NordColor,  // #d8dee9 - Dark foreground
    pub nord5: NordColor,  // #e5e9f0 - Medium foreground
    pub nord6: NordColor,  // #eceff4 - Light foreground (text)

    // Frost (blue accent colors)
    pub nord7: NordColor,  // #8fbcbb - Teal
    pub nord8: NordColor,  // #88c0d0 - Cyan (primary accent)
    pub nord9: NordColor,  // #81a1c1 - Blue
    pub nord10: NordColor, // #5e81ac - Dark blue

    // Aurora (colorful accents)
    pub nord11: NordColor, // #bf616a - Red (errors, close)
    pub nord12: NordColor, // #d08770 - Orange (warnings)
    pub nord13: NordColor, // #ebcb8b - Yellow (highlights)
    pub nord14: NordColor, // #a3be8c - Green (success)
    pub nord15: NordColor, // #b48ead - Purple (special)
}

impl Default for NordColors {
    fn default() -> Self {
        Self {
            // Polar Night
            nord0: NordColor::rgb(0x2e, 0x34, 0x40),
            nord1: NordColor::rgb(0x3b, 0x42, 0x52),
            nord2: NordColor::rgb(0x43, 0x4c, 0x5e),
            nord3: NordColor::rgb(0x4c, 0x56, 0x6a),

            // Snow Storm
            nord4: NordColor::rgb(0xd8, 0xde, 0xe9),
            nord5: NordColor::rgb(0xe5, 0xe9, 0xf0),
            nord6: NordColor::rgb(0xec, 0xef, 0xf4),

            // Frost
            nord7: NordColor::rgb(0x8f, 0xbc, 0xbb),
            nord8: NordColor::rgb(0x88, 0xc0, 0xd0),
            nord9: NordColor::rgb(0x81, 0xa1, 0xc1),
            nord10: NordColor::rgb(0x5e, 0x81, 0xac),

            // Aurora
            nord11: NordColor::rgb(0xbf, 0x61, 0x6a),
            nord12: NordColor::rgb(0xd0, 0x87, 0x70),
            nord13: NordColor::rgb(0xeb, 0xcb, 0x8b),
            nord14: NordColor::rgb(0xa3, 0xbe, 0x8c),
            nord15: NordColor::rgb(0xb4, 0x8e, 0xad),
        }
    }
}

/// Font configuration
#[derive(Debug, Clone)]
pub struct FontConfig {
    /// Title font family
    pub title_family: String,
    /// UI font family
    pub ui_family: String,
    /// Font weight for titles
    pub title_weight: u32,
    /// Font weight for UI
    pub ui_weight: u32,
    pub title_size: u32,
    pub input_size: u32,
    pub item_size: u32,
    pub description_size: u32,
    pub hint_size: u32,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            title_family: "sans-serif".to_string(),
            ui_family: "sans-serif".to_string(),
            title_weight: 600,
            ui_weight: 400,
            title_size: 18,
            input_size: 16,
            item_size: 14,
            description_size: 12,
            hint_size: 11,
        }
    }
}

/// Complete Nord theme configuration
#[derive(Debug, Clone)]
pub struct NordTheme {
    pub colors: NordColors,
    pub fonts: FontConfig,
    pub border_width: u32,
    pub gap_width: u32,
    /// Window opacity (0.0 - 1.0, for future transparency support)
    pub opacity: f32,
    /// Enable window shadows (not yet implemented)
    pub shadow_enabled: bool,
    /// Animation duration in milliseconds (for future transitions)
    pub animation_duration: u32,
}

impl Default for NordTheme {
    fn default() -> Self {
        Self {
            colors: NordColors::default(),
            fonts: FontConfig::default(),
            border_width: 2,
            gap_width: 10,
            opacity: 1.0,
            shadow_enabled: false,
            animation_duration: 200,
        }
    }
}

impl NordTheme {
    /// Create a new theme with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the background color (nord0)
    pub fn background(&self) -> NordColor {
        self.colors.nord0
    }

    /// Get the focused border color (nord8 - cyan)
    pub fn focused_border(&self) -> NordColor {
        self.colors.nord8
    }

    /// Get the unfocused border color (nord3 - gray)
    pub fn unfocused_border(&self) -> NordColor {
        self.colors.nord3
    }

    /// Get the primary text color (nord6 - white)
    pub fn text(&self) -> NordColor {
        self.colors.nord6
    }

    /// Get the secondary text color (nord4)
    pub fn text_secondary(&self) -> NordColor {
        self.colors.nord4
    }

    /// Get the accent color (nord8 - cyan)
    pub fn accent(&self) -> NordColor {
        self.colors.nord8
    }

    /// Get the error color (nord11 - red)
    pub fn error(&self) -> NordColor {
        self.colors.nord11
    }

    /// Get the success color (nord14 - green)
    pub fn success(&self) -> NordColor {
        self.colors.nord14
    }

    /// Get the warning color (nord12 - orange)
    pub fn warning(&self) -> NordColor {
        self.colors.nord12
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nord_color_conversion() {
        let color = NordColor::rgb(0x88, 0xc0, 0xd0);
        let f32_array = color.to_f32_array();

        // Check approximate conversion (allow small floating point errors)
        assert!((f32_array[0] - 0.533).abs() < 0.01);
        assert!((f32_array[1] - 0.753).abs() < 0.01);
        assert!((f32_array[2] - 0.815).abs() < 0.01);
        assert_eq!(f32_array[3], 1.0);
    }

    #[test]
    fn test_theme_defaults() {
        let theme = NordTheme::new();
        assert_eq!(theme.border_width, 2);
        assert_eq!(theme.gap_width, 10);
        assert_eq!(theme.background(), theme.colors.nord0);
        assert_eq!(theme.focused_border(), theme.colors.nord8);
    }
}
