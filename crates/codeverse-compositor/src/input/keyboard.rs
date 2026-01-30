use crate::compositor::CodeVerseCompositor;
use codeverse_window::{Direction, LayoutMode, Orientation, WindowTreeExt};
use smithay::input::{
    keyboard::{KeyboardTarget, KeysymHandle, ModifiersState},
    Seat,
};
use std::process::Command;
use tracing::{debug, info, warn};
use xkbcommon::xkb::{self, Keysym};

/// Handle keyboard input for the compositor
pub fn handle_keyboard_shortcut<BackendData: 'static>(
    compositor: &mut CodeVerseCompositor<BackendData>,
    keysym: Keysym,
    modifiers: ModifiersState,
) -> bool {
    // If launcher is active, handle launcher input first
    if compositor.launcher_active {
        return handle_launcher_input(compositor, keysym, modifiers);
    }

    // Check if Super (Logo/Mod) key is pressed
    let logo_pressed = modifiers.logo;
    let shift_pressed = modifiers.shift;

    // Super+d: Toggle launcher
    if logo_pressed && !shift_pressed && keysym == Keysym::d {
        info!("Toggling launcher (Super+d pressed)");
        compositor.toggle_launcher();
        return true;
    }

    // Super+Shift+Q: Quit compositor
    if logo_pressed && shift_pressed && keysym == Keysym::q {
        info!("Quit shortcut pressed, exiting compositor");
        compositor.running = false;
        return true;
    }

    // Super+Shift+R: Reload configuration
    if logo_pressed && shift_pressed && keysym == Keysym::r {
        info!("Reload config shortcut pressed");
        compositor.reload_config();
        return true;
    }

    // Navigation: Super+h/j/k/l
    if logo_pressed && !shift_pressed {
        let direction = match keysym {
            Keysym::h => Some(Direction::Left),
            Keysym::j => Some(Direction::Down),
            Keysym::k => Some(Direction::Up),
            Keysym::l => Some(Direction::Right),
            _ => None,
        };

        if let Some(dir) = direction {
            debug!("Navigation shortcut: {:?}", dir);
            compositor.window_tree.navigate_focus(dir);
            return true;
        }
    }

    // Split: Super+b (horizontal) / Super+v (vertical)
    if logo_pressed && !shift_pressed {
        let orientation = match keysym {
            Keysym::b => Some(Orientation::Horizontal),
            Keysym::v => Some(Orientation::Vertical),
            _ => None,
        };

        if let Some(orient) = orientation {
            debug!("Split shortcut: {:?}", orient);
            if let Err(e) = compositor.window_tree.split_focused(orient) {
                tracing::warn!("Failed to split: {}", e);
            }
            return true;
        }
    }

    // Workspace switching: Super+1-0
    if logo_pressed && !shift_pressed {
        if let Some(workspace_num) = keysym_to_workspace_num(keysym) {
            debug!("Switching to workspace {}", workspace_num);
            if let Some(ref mut manager) = compositor.workspace_manager {
                manager.switch_to_workspace(workspace_num);

                // Recalculate layout for the new workspace
                if let Some(workspace_id) = manager.active_workspace() {
                    // Get screen geometry from backend
                    // For now, use a default - this will be updated in rendering
                    let screen_geometry = codeverse_window::Rectangle::new(0, 0, 1920, 1080);
                    manager.layout_active_workspace(&mut compositor.window_tree, screen_geometry);
                }
            }
            return true;
        }
    }

    // Move window to workspace: Super+Shift+1-0
    if logo_pressed && shift_pressed {
        if let Some(workspace_num) = keysym_to_workspace_num(keysym) {
            debug!("Moving window to workspace {}", workspace_num);
            if let (Some(focused_id), Some(ref mut manager)) =
                (compositor.window_tree.focused(), compositor.workspace_manager.as_mut())
            {
                if let Err(e) = manager.move_window_to_workspace(
                    &mut compositor.window_tree,
                    focused_id,
                    workspace_num,
                ) {
                    tracing::warn!("Failed to move window to workspace: {}", e);
                }
            }
            return true;
        }
    }

    // Layout switching: Super+e/w/s/t
    if logo_pressed && !shift_pressed {
        let layout_mode = match keysym {
            Keysym::e => Some(LayoutMode::SplitH),  // Horizontal split
            Keysym::w => Some(LayoutMode::SplitV),  // Vertical split
            Keysym::s => Some(LayoutMode::Stacking), // Stacking
            Keysym::t => Some(LayoutMode::Tabbed),   // Tabbed
            _ => None,
        };

        if let Some(layout) = layout_mode {
            debug!("Layout switching shortcut: {:?}", layout);
            if let Err(e) = compositor.window_tree.change_layout(layout) {
                tracing::warn!("Failed to change layout: {}", e);
            } else {
                // Recalculate layout after changing mode
                if let Some(ref mut manager) = compositor.workspace_manager {
                    if let Some(workspace_id) = manager.active_workspace() {
                        // Get screen geometry from backend - using a default for now
                        let screen_geometry = codeverse_window::Rectangle::new(0, 0, 1920, 1080);
                        manager.layout_active_workspace(&mut compositor.window_tree, screen_geometry);
                    }
                }
            }
            return true;
        }
    }

    // Super+Shift+Space: Toggle floating mode
    if logo_pressed && shift_pressed && keysym == Keysym::space {
        debug!("Toggle floating shortcut");
        if let Some(focused_id) = compositor.window_tree.focused() {
            // Get screen geometry
            let screen_geometry = codeverse_window::Rectangle::new(0, 0, 1920, 1080);

            if let Err(e) = compositor.floating_manager.toggle_floating(
                &mut compositor.window_tree,
                focused_id,
                screen_geometry,
            ) {
                tracing::warn!("Failed to toggle floating: {}", e);
            } else {
                // Recalculate layout after toggle
                if let Some(ref mut manager) = compositor.workspace_manager {
                    if let Some(workspace_id) = manager.active_workspace() {
                        manager.layout_active_workspace(&mut compositor.window_tree, screen_geometry);
                    }
                }
            }
        }
        return true;
    }

    // Super+Shift+C: Close focused window
    if logo_pressed && shift_pressed && keysym == Keysym::c {
        debug!("Close window shortcut");
        if let Some(focused_id) = compositor.window_tree.focused() {
            // Find the toplevel surface and close it
            if let Some(container) = compositor.window_tree.get(focused_id) {
                if let Some(ref window) = container.window {
                    window.send_close();
                }
            }
        }
        return true;
    }

    // F12: Spawn test window (no modifier needed to avoid conflicts)
    if keysym == Keysym::F12 {
        info!("Spawning test window (F12 pressed)");
        spawn_test_window(compositor.socket_name.as_deref());
        return true;
    }

    false // Shortcut not handled
}

/// Spawn a test window for testing the compositor
/// Tries multiple terminal emulators in order of preference
fn spawn_test_window(socket_name: Option<&str>) {
    let terminals = [
        "weston-terminal",
        "alacritty",
        "kitty",
        "foot",
        "gnome-terminal",
        "konsole",
        "xterm",
    ];

    for terminal in &terminals {
        let mut cmd = Command::new(terminal);

        // Set WAYLAND_DISPLAY to connect to our compositor
        if let Some(socket) = socket_name {
            cmd.env("WAYLAND_DISPLAY", socket);
            info!("Setting WAYLAND_DISPLAY={} for spawned terminal", socket);
        }

        match cmd.spawn() {
            Ok(child) => {
                info!("Spawned test window: {} (PID: {})", terminal, child.id());
                return;
            }
            Err(_) => {
                // Try next terminal
                continue;
            }
        }
    }

    warn!("Failed to spawn test window - no terminal emulator found. Tried: {:?}", terminals);
}

/// Handle keyboard input when launcher is active
fn handle_launcher_input<BackendData: 'static>(
    compositor: &mut CodeVerseCompositor<BackendData>,
    keysym: Keysym,
    modifiers: ModifiersState,
) -> bool {
    // Escape: Close launcher
    if keysym == Keysym::Escape {
        debug!("Closing launcher (Escape pressed)");
        compositor.launcher_active = false;
        return true;
    }

    // Enter: Launch selected app
    if keysym == Keysym::Return || keysym == Keysym::KP_Enter {
        debug!("Launching selected app (Enter pressed)");
        if let Err(e) = compositor.launch_selected_app() {
            warn!("Failed to launch app: {}", e);
        }
        return true;
    }

    // Arrow keys: Navigate selection
    if keysym == Keysym::Up {
        if let Some(ref mut launcher) = compositor.launcher {
            launcher.select_previous();
            debug!("Launcher: selected previous");
        }
        return true;
    }

    if keysym == Keysym::Down {
        if let Some(ref mut launcher) = compositor.launcher {
            launcher.select_next();
            debug!("Launcher: selected next");
        }
        return true;
    }

    // Backspace: Remove last character
    if keysym == Keysym::BackSpace {
        if let Some(ref mut launcher) = compositor.launcher {
            launcher.pop_char();
            debug!("Launcher query: {}", launcher.query());
        }
        return true;
    }

    // Super+d: Toggle launcher off (same key that opened it)
    if modifiers.logo && keysym == Keysym::d {
        debug!("Toggling launcher off (Super+d pressed)");
        compositor.launcher_active = false;
        return true;
    }

    // Typing: Add character to search query
    // Convert keysym to character if it's a printable character
    if let Some(ch) = keysym_to_char(keysym, modifiers.shift) {
        if let Some(ref mut launcher) = compositor.launcher {
            launcher.push_char(ch);
            debug!("Launcher query: {}", launcher.query());
        }
        return true;
    }

    false // Not handled
}

/// Convert keysym to character (basic ASCII only for MVP)
fn keysym_to_char(keysym: Keysym, shift: bool) -> Option<char> {
    // Letters
    if keysym >= Keysym::a && keysym <= Keysym::z {
        let offset = keysym.raw() - Keysym::a.raw();
        let ch = if shift {
            (b'A' + offset as u8) as char
        } else {
            (b'a' + offset as u8) as char
        };
        return Some(ch);
    }

    // Numbers (top row)
    if keysym >= Keysym::_0 && keysym <= Keysym::_9 {
        if shift {
            // Shift + number gives symbols
            let symbols = [')', '!', '@', '#', '$', '%', '^', '&', '*', '('];
            let offset = (keysym.raw() - Keysym::_0.raw()) as usize;
            return symbols.get(offset).copied();
        } else {
            let offset = keysym.raw() - Keysym::_0.raw();
            return Some((b'0' + offset as u8) as char);
        }
    }

    // Space
    if keysym == Keysym::space {
        return Some(' ');
    }

    // Common punctuation
    match keysym {
        Keysym::minus => Some(if shift { '_' } else { '-' }),
        Keysym::equal => Some(if shift { '+' } else { '=' }),
        Keysym::bracketleft => Some(if shift { '{' } else { '[' }),
        Keysym::bracketright => Some(if shift { '}' } else { ']' }),
        Keysym::semicolon => Some(if shift { ':' } else { ';' }),
        Keysym::apostrophe => Some(if shift { '"' } else { '\'' }),
        Keysym::comma => Some(if shift { '<' } else { ',' }),
        Keysym::period => Some(if shift { '>' } else { '.' }),
        Keysym::slash => Some(if shift { '?' } else { '/' }),
        Keysym::backslash => Some(if shift { '|' } else { '\\' }),
        Keysym::grave => Some(if shift { '~' } else { '`' }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compositor::CodeVerseCompositor;
    use smithay::{
        input::keyboard::ModifiersState,
        reexports::calloop::EventLoop,
        reexports::wayland_server::Display,
    };

    // Helper to create a test compositor
    fn create_test_compositor() -> CodeVerseCompositor<()> {
        let mut event_loop: EventLoop<CodeVerseCompositor<()>> = EventLoop::try_new().unwrap();
        let loop_handle = event_loop.handle();
        let mut display: Display<CodeVerseCompositor<()>> = Display::new().unwrap();

        CodeVerseCompositor::new(&mut display, loop_handle, ())
    }

    // Helper to create modifier state
    fn modifiers(logo: bool, shift: bool, ctrl: bool, alt: bool) -> ModifiersState {
        ModifiersState {
            ctrl,
            alt,
            shift,
            caps_lock: false,
            logo,
            num_lock: false,
            iso_level3_shift: false,
            iso_level5_shift: false,
            serialized: Default::default(),
        }
    }

    #[test]
    fn test_quit_shortcut() {
        let mut compositor = create_test_compositor();
        assert!(compositor.running); // Should start running

        // Super+Shift+Q should quit
        let result = handle_keyboard_shortcut(
            &mut compositor,
            Keysym::q,
            modifiers(true, true, false, false),
        );

        assert!(result); // Shortcut should be handled
        assert!(!compositor.running); // Should be set to not running
    }

    #[test]
    fn test_navigation_shortcuts() {
        let mut compositor = create_test_compositor();
        compositor.init_workspace_manager();

        // Super+h/j/k/l should be recognized (even if no windows to navigate)
        let nav_keys = [
            (Keysym::h, "left"),
            (Keysym::j, "down"),
            (Keysym::k, "up"),
            (Keysym::l, "right"),
        ];

        for (key, _direction) in nav_keys {
            let result = handle_keyboard_shortcut(
                &mut compositor,
                key,
                modifiers(true, false, false, false),
            );
            assert!(result, "Navigation key {:?} should be handled", key);
        }
    }

    #[test]
    fn test_split_shortcuts() {
        let mut compositor = create_test_compositor();
        compositor.init_workspace_manager();

        // Super+b should attempt horizontal split (may fail without windows)
        let result_b = handle_keyboard_shortcut(
            &mut compositor,
            Keysym::b,
            modifiers(true, false, false, false),
        );
        assert!(result_b, "Super+b should be handled");

        // Super+v should attempt vertical split
        let result_v = handle_keyboard_shortcut(
            &mut compositor,
            Keysym::v,
            modifiers(true, false, false, false),
        );
        assert!(result_v, "Super+v should be handled");
    }

    #[test]
    fn test_workspace_switching() {
        let mut compositor = create_test_compositor();
        compositor.init_workspace_manager();

        // Super+1 through Super+0 should switch workspaces
        let workspace_keys = [
            Keysym::_1, Keysym::_2, Keysym::_3, Keysym::_4, Keysym::_5,
            Keysym::_6, Keysym::_7, Keysym::_8, Keysym::_9, Keysym::_0,
        ];

        for (i, key) in workspace_keys.iter().enumerate() {
            let result = handle_keyboard_shortcut(
                &mut compositor,
                *key,
                modifiers(true, false, false, false),
            );
            assert!(result, "Workspace key {:?} should be handled", key);

            // Verify workspace was switched
            if let Some(ref manager) = compositor.workspace_manager {
                let expected = if i == 9 { 10 } else { i + 1 }; // 0 maps to workspace 10
                assert_eq!(manager.active_workspace_num(), expected);
            }
        }
    }

    #[test]
    fn test_layout_switching() {
        let mut compositor = create_test_compositor();
        compositor.init_workspace_manager();

        // Layout switching keys (may fail without windows, but should be recognized)
        let layout_keys = [
            (Keysym::e, "horizontal split"),
            (Keysym::w, "vertical split"),
            (Keysym::s, "stacking"),
            (Keysym::t, "tabbed"),
        ];

        for (key, _layout) in layout_keys {
            let result = handle_keyboard_shortcut(
                &mut compositor,
                key,
                modifiers(true, false, false, false),
            );
            assert!(result, "Layout key {:?} should be handled", key);
        }
    }

    #[test]
    fn test_f12_test_window() {
        let mut compositor = create_test_compositor();

        // F12 should spawn test window (handled even if spawn fails)
        let result = handle_keyboard_shortcut(
            &mut compositor,
            Keysym::F12,
            modifiers(false, false, false, false),
        );
        assert!(result, "F12 should be handled");
    }

    #[test]
    fn test_close_window_shortcut() {
        let mut compositor = create_test_compositor();

        // Super+Shift+C should be recognized (even without focused window)
        let result = handle_keyboard_shortcut(
            &mut compositor,
            Keysym::c,
            modifiers(true, true, false, false),
        );
        assert!(result, "Super+Shift+C should be handled");
    }

    #[test]
    fn test_unhandled_shortcuts() {
        let mut compositor = create_test_compositor();

        // Random key without modifier should not be handled
        let result = handle_keyboard_shortcut(
            &mut compositor,
            Keysym::a,
            modifiers(false, false, false, false),
        );
        assert!(!result, "Random key 'a' without modifiers should not be handled");

        // Super+Z (not bound) should not be handled
        let result = handle_keyboard_shortcut(
            &mut compositor,
            Keysym::z,
            modifiers(true, false, false, false),
        );
        assert!(!result, "Super+Z (unbound) should not be handled");
    }

    #[test]
    fn test_keysym_to_workspace_num() {
        assert_eq!(keysym_to_workspace_num(Keysym::_1), Some(1));
        assert_eq!(keysym_to_workspace_num(Keysym::_2), Some(2));
        assert_eq!(keysym_to_workspace_num(Keysym::_9), Some(9));
        assert_eq!(keysym_to_workspace_num(Keysym::_0), Some(10));
        assert_eq!(keysym_to_workspace_num(Keysym::a), None);
    }

    #[test]
    fn test_modifier_combinations() {
        let mut compositor = create_test_compositor();

        // Super alone + key should work
        let result = handle_keyboard_shortcut(
            &mut compositor,
            Keysym::h,
            modifiers(true, false, false, false),
        );
        assert!(result, "Super+h should be handled");

        // Super+Shift + key should work
        let result = handle_keyboard_shortcut(
            &mut compositor,
            Keysym::q,
            modifiers(true, true, false, false),
        );
        assert!(result, "Super+Shift+q should be handled");

        // Other modifiers without Super should not trigger shortcuts
        let result = handle_keyboard_shortcut(
            &mut compositor,
            Keysym::q,
            modifiers(false, true, true, false), // Shift+Ctrl but no Super
        );
        assert!(!result, "Shift+Ctrl+q (no Super) should not be handled");
    }
}

/// Convert keysym to workspace number (1-10)
fn keysym_to_workspace_num(keysym: Keysym) -> Option<usize> {
    match keysym {
        Keysym::_1 => Some(1),
        Keysym::_2 => Some(2),
        Keysym::_3 => Some(3),
        Keysym::_4 => Some(4),
        Keysym::_5 => Some(5),
        Keysym::_6 => Some(6),
        Keysym::_7 => Some(7),
        Keysym::_8 => Some(8),
        Keysym::_9 => Some(9),
        Keysym::_0 => Some(10),
        _ => None,
    }
}
