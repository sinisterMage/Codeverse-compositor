use crate::compositor::CodeVerseCompositor;
use codeverse_window::{MouseOperation, ResizeEdge};
use smithay::input::pointer::AxisFrame;
use smithay::utils::{Logical, Point, Serial, SERIAL_COUNTER};
use tracing::debug;

/// Handle pointer button press/release
pub fn handle_pointer_button<BackendData: 'static>(
    compositor: &mut CodeVerseCompositor<BackendData>,
    button: u32,
    state: smithay::backend::input::ButtonState,
    _serial: Serial,
    _time: u32,
    location: Point<f64, Logical>,
) {
    let modifiers = compositor
        .seat
        .get_keyboard()
        .unwrap()
        .modifier_state();

    let logo_pressed = modifiers.logo;
    let x = location.x as i32;
    let y = location.y as i32;

    // Mouse button constants
    const BTN_LEFT: u32 = 0x110;
    const BTN_RIGHT: u32 = 0x111;

    if state == smithay::backend::input::ButtonState::Pressed {
        // Find window under cursor
        if let Some(window_id) = compositor.floating_manager.find_window_at(&compositor.window_tree, x, y) {
            // Set focus to clicked window
            compositor.window_tree.set_focused(Some(window_id));

            // Raise window to top
            compositor.floating_manager.raise_window(window_id);

            // Super+LeftClick: Start moving
            if logo_pressed && button == BTN_LEFT {
                debug!("Starting window move with Super+LeftClick");
                if let Err(e) = compositor.floating_manager.start_move(
                    &compositor.window_tree,
                    window_id,
                    x,
                    y,
                ) {
                    tracing::warn!("Failed to start move: {}", e);
                }
                return;
            }

            // Super+RightClick: Start resizing
            if logo_pressed && button == BTN_RIGHT {
                debug!("Starting window resize with Super+RightClick");

                // Detect which edge to resize from
                let edge = compositor
                    .floating_manager
                    .detect_resize_edge(&compositor.window_tree, window_id, x, y)
                    .unwrap_or(ResizeEdge::BottomRight); // Default to bottom-right if not on edge

                if let Err(e) = compositor.floating_manager.start_resize(
                    &compositor.window_tree,
                    window_id,
                    x,
                    y,
                    edge,
                ) {
                    tracing::warn!("Failed to start resize: {}", e);
                }
                return;
            }

            // Regular click on title bar: Start moving (without modifier)
            if compositor.floating_manager.is_in_title_bar(&compositor.window_tree, window_id, x, y) {
                debug!("Starting window move by dragging title bar");
                if let Err(e) = compositor.floating_manager.start_move(
                    &compositor.window_tree,
                    window_id,
                    x,
                    y,
                ) {
                    tracing::warn!("Failed to start move: {}", e);
                }
                return;
            }
        }
    } else {
        // Button released: finish operation
        if !matches!(compositor.floating_manager.current_operation(), MouseOperation::None) {
            debug!("Finishing mouse operation");
            compositor.floating_manager.finish_operation();
        }
    }
}

/// Handle pointer motion
pub fn handle_pointer_motion<BackendData: 'static>(
    compositor: &mut CodeVerseCompositor<BackendData>,
    location: Point<f64, Logical>,
    _time: u32,
) {
    let x = location.x as i32;
    let y = location.y as i32;

    // Update ongoing operation
    if !matches!(compositor.floating_manager.current_operation(), MouseOperation::None) {
        if let Err(e) = compositor.floating_manager.update_operation(&mut compositor.window_tree, x, y) {
            tracing::warn!("Failed to update operation: {}", e);
        }
        return;
    }

    // Focus follows mouse: when enabled, move keyboard focus to
    // whichever window the pointer enters.
    if compositor.config.general.focus_follows_mouse {
        if let Some(window_id) = compositor.window_under(location) {
            let current_focus = compositor.window_tree.focused();
            if current_focus != Some(window_id) {
                compositor.window_tree.set_focused(Some(window_id));
                compositor.update_window_border_colors();

                let serial = SERIAL_COUNTER.next_serial();
                let kb_surface = compositor.window_tree.get(window_id)
                    .and_then(|c| c.window.as_ref())
                    .map(|t| t.wl_surface().clone());
                if let Some(surface) = kb_surface {
                    let keyboard = compositor.seat.get_keyboard().unwrap();
                    keyboard.set_focus(
                        compositor,
                        Some(crate::focus::KeyboardFocusTarget::Surface(surface)),
                        serial,
                    );
                }
            }
        }
    }
}

/// Handle pointer axis (scroll) events
pub fn handle_pointer_axis<BackendData: 'static>(
    _compositor: &mut CodeVerseCompositor<BackendData>,
    _frame: AxisFrame,
) {
    // Axis events not used for now
}
