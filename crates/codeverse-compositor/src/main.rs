mod backend;
mod compositor;
mod focus;
mod handlers;
mod input;
mod render;

use std::env;
use tracing::info;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    info!("CodeVerse Compositor starting...");

    // Auto-detect backend based on environment
    // Use winit if DISPLAY or WAYLAND_DISPLAY is set (running in X11/Wayland session)
    // Otherwise use DRM (running directly on TTY)
    if should_use_winit() {
        info!("Using Winit backend (nested session)");
        backend::init_winit()?;
    } else {
        info!("Using DRM backend (TTY)");
        backend::init_drm()?;
    }

    Ok(())
}

/// Determine if we should use the winit backend
/// Returns true if we're running in an existing display server (X11 or Wayland)
fn should_use_winit() -> bool {
    env::var("DISPLAY").is_ok() || env::var("WAYLAND_DISPLAY").is_ok()
}
