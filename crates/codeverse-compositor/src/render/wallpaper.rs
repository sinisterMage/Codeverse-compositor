//! Wallpaper rendering module
//!
//! Provides functionality for loading, scaling, and caching wallpaper images
//! for rendering as compositor backgrounds.

use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, warn};

/// Scaling modes for wallpaper images
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScaleMode {
    /// Scale to fill the entire screen, cropping if necessary
    Fill,
    /// Scale to fit within the screen, may have letterboxing
    Fit,
    /// Stretch to exactly fill the screen (may distort aspect ratio)
    Stretch,
    /// Center the image at original size
    Center,
    /// Tile the image to fill the screen
    Tile,
}

impl ScaleMode {
    /// Parse a scale mode from a string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "fill" => ScaleMode::Fill,
            "fit" => ScaleMode::Fit,
            "stretch" => ScaleMode::Stretch,
            "center" => ScaleMode::Center,
            "tile" => ScaleMode::Tile,
            _ => ScaleMode::Fill, // Default to fill
        }
    }
}

/// Cache key for wallpaper textures
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WallpaperCacheKey {
    pub path: String,
    pub screen_width: u32,
    pub screen_height: u32,
    pub mode: ScaleMode,
}

/// Cached wallpaper data (RGBA pixels)
pub struct CachedWallpaper {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Wallpaper cache for storing loaded and scaled textures
pub struct WallpaperCache {
    cache: HashMap<WallpaperCacheKey, CachedWallpaper>,
}

impl WallpaperCache {
    /// Create a new empty wallpaper cache
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Clear the entire cache
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Get a cached wallpaper if it exists
    pub fn get(&self, key: &WallpaperCacheKey) -> Option<&CachedWallpaper> {
        self.cache.get(key)
    }

    /// Insert a wallpaper into the cache
    pub fn insert(&mut self, key: WallpaperCacheKey, wallpaper: CachedWallpaper) {
        self.cache.insert(key, wallpaper);
    }

    /// Check if a wallpaper is cached
    pub fn contains(&self, key: &WallpaperCacheKey) -> bool {
        self.cache.contains_key(key)
    }
}

impl Default for WallpaperCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Load a wallpaper image from a file path
///
/// Returns RGBA pixel data and dimensions, or None if loading fails.
#[cfg(feature = "wallpaper")]
pub fn load_wallpaper_image(path: &Path) -> Option<(Vec<u8>, u32, u32)> {
    use image::GenericImageView;

    match image::open(path) {
        Ok(img) => {
            let (width, height) = img.dimensions();
            let rgba = img.to_rgba8();
            debug!("Loaded wallpaper {}x{} from {:?}", width, height, path);
            Some((rgba.into_raw(), width, height))
        }
        Err(e) => {
            warn!("Failed to load wallpaper from {:?}: {}", path, e);
            None
        }
    }
}

/// Stub implementation when wallpaper feature is disabled
#[cfg(not(feature = "wallpaper"))]
pub fn load_wallpaper_image(_path: &Path) -> Option<(Vec<u8>, u32, u32)> {
    warn!("Wallpaper support not enabled (missing 'wallpaper' feature)");
    None
}

/// Scale wallpaper image to target dimensions with the given mode
#[cfg(feature = "wallpaper")]
pub fn scale_wallpaper(
    data: &[u8],
    src_width: u32,
    src_height: u32,
    target_width: u32,
    target_height: u32,
    mode: ScaleMode,
) -> (Vec<u8>, u32, u32) {
    use image::{ImageBuffer, Rgba, imageops::FilterType};

    let src_image: ImageBuffer<Rgba<u8>, _> =
        ImageBuffer::from_raw(src_width, src_height, data.to_vec())
            .expect("Invalid image data");

    match mode {
        ScaleMode::Stretch => {
            // Simply resize to target dimensions
            let resized = image::imageops::resize(&src_image, target_width, target_height, FilterType::Lanczos3);
            (resized.into_raw(), target_width, target_height)
        }
        ScaleMode::Fill => {
            // Scale to fill, maintaining aspect ratio, then crop
            let src_aspect = src_width as f32 / src_height as f32;
            let target_aspect = target_width as f32 / target_height as f32;

            let (scale_width, scale_height) = if src_aspect > target_aspect {
                // Source is wider, scale by height
                let scale = target_height as f32 / src_height as f32;
                ((src_width as f32 * scale) as u32, target_height)
            } else {
                // Source is taller, scale by width
                let scale = target_width as f32 / src_width as f32;
                (target_width, (src_height as f32 * scale) as u32)
            };

            let resized = image::imageops::resize(&src_image, scale_width, scale_height, FilterType::Lanczos3);

            // Crop to target size (centered)
            let x_offset = (scale_width.saturating_sub(target_width)) / 2;
            let y_offset = (scale_height.saturating_sub(target_height)) / 2;

            let cropped = image::imageops::crop_imm(&resized, x_offset, y_offset, target_width, target_height);
            (cropped.to_image().into_raw(), target_width, target_height)
        }
        ScaleMode::Fit => {
            // Scale to fit within target, maintaining aspect ratio (letterboxing)
            let src_aspect = src_width as f32 / src_height as f32;
            let target_aspect = target_width as f32 / target_height as f32;

            let (fit_width, fit_height) = if src_aspect > target_aspect {
                // Source is wider, fit by width
                let scale = target_width as f32 / src_width as f32;
                (target_width, (src_height as f32 * scale) as u32)
            } else {
                // Source is taller, fit by height
                let scale = target_height as f32 / src_height as f32;
                ((src_width as f32 * scale) as u32, target_height)
            };

            let resized = image::imageops::resize(&src_image, fit_width, fit_height, FilterType::Lanczos3);

            // Create output image with background color (nord0: #2e3440)
            let mut output: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_pixel(
                target_width,
                target_height,
                Rgba([0x2e, 0x34, 0x40, 0xff]),
            );

            // Center the resized image
            let x_offset = (target_width.saturating_sub(fit_width)) / 2;
            let y_offset = (target_height.saturating_sub(fit_height)) / 2;

            image::imageops::overlay(&mut output, &resized, x_offset as i64, y_offset as i64);

            (output.into_raw(), target_width, target_height)
        }
        ScaleMode::Center => {
            // Center at original size
            let mut output: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_pixel(
                target_width,
                target_height,
                Rgba([0x2e, 0x34, 0x40, 0xff]),
            );

            let x_offset = (target_width as i64 - src_width as i64) / 2;
            let y_offset = (target_height as i64 - src_height as i64) / 2;

            image::imageops::overlay(&mut output, &src_image, x_offset, y_offset);

            (output.into_raw(), target_width, target_height)
        }
        ScaleMode::Tile => {
            // Tile the image to fill the screen
            let mut output: ImageBuffer<Rgba<u8>, _> = ImageBuffer::new(target_width, target_height);

            for y in (0..target_height).step_by(src_height as usize) {
                for x in (0..target_width).step_by(src_width as usize) {
                    image::imageops::overlay(&mut output, &src_image, x as i64, y as i64);
                }
            }

            (output.into_raw(), target_width, target_height)
        }
    }
}

/// Stub implementation when wallpaper feature is disabled
#[cfg(not(feature = "wallpaper"))]
pub fn scale_wallpaper(
    data: &[u8],
    src_width: u32,
    src_height: u32,
    _target_width: u32,
    _target_height: u32,
    _mode: ScaleMode,
) -> (Vec<u8>, u32, u32) {
    // Return original data unchanged
    (data.to_vec(), src_width, src_height)
}

/// Load and scale a wallpaper, using the cache if available
///
/// Returns true if the wallpaper was successfully loaded/cached, false otherwise.
/// Use `cache.get(&key)` afterward to retrieve the wallpaper.
pub fn load_cached_wallpaper(
    cache: &mut WallpaperCache,
    path: &str,
    screen_width: u32,
    screen_height: u32,
    mode: ScaleMode,
) -> bool {
    let key = WallpaperCacheKey {
        path: path.to_string(),
        screen_width,
        screen_height,
        mode,
    };

    // Check if already cached
    if cache.contains(&key) {
        return true;
    }

    // Load the image
    let path_ref = Path::new(path);
    let Some((data, width, height)) = load_wallpaper_image(path_ref) else {
        return false;
    };

    // Scale the image
    let (scaled_data, scaled_width, scaled_height) =
        scale_wallpaper(&data, width, height, screen_width, screen_height, mode);

    // Cache it
    let wallpaper = CachedWallpaper {
        data: scaled_data,
        width: scaled_width,
        height: scaled_height,
    };
    cache.insert(key, wallpaper);

    true
}

/// Get the cache key for a wallpaper
pub fn make_wallpaper_key(
    path: &str,
    screen_width: u32,
    screen_height: u32,
    mode: ScaleMode,
) -> WallpaperCacheKey {
    WallpaperCacheKey {
        path: path.to_string(),
        screen_width,
        screen_height,
        mode,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scale_mode_from_str() {
        assert_eq!(ScaleMode::from_str("fill"), ScaleMode::Fill);
        assert_eq!(ScaleMode::from_str("FIT"), ScaleMode::Fit);
        assert_eq!(ScaleMode::from_str("Stretch"), ScaleMode::Stretch);
        assert_eq!(ScaleMode::from_str("center"), ScaleMode::Center);
        assert_eq!(ScaleMode::from_str("tile"), ScaleMode::Tile);
        assert_eq!(ScaleMode::from_str("unknown"), ScaleMode::Fill); // Default
    }

    #[test]
    fn test_wallpaper_cache() {
        let mut cache = WallpaperCache::new();
        let key = WallpaperCacheKey {
            path: "/test/path.png".to_string(),
            screen_width: 1920,
            screen_height: 1080,
            mode: ScaleMode::Fill,
        };

        assert!(!cache.contains(&key));

        let wallpaper = CachedWallpaper {
            data: vec![0; 1920 * 1080 * 4],
            width: 1920,
            height: 1080,
        };
        cache.insert(key.clone(), wallpaper);

        assert!(cache.contains(&key));
        assert!(cache.get(&key).is_some());

        cache.clear();
        assert!(!cache.contains(&key));
    }
}
