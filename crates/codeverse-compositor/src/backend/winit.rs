use crate::compositor::{ClientState, CodeVerseCompositor};
use crate::input::{handle_keyboard_shortcut, handle_pointer_axis, handle_pointer_button, handle_pointer_motion};
use crate::render::{create_border_elements, BorderRenderElement, load_cached_wallpaper, make_wallpaper_key};
use smithay::{
    backend::{
        input::{
            AbsolutePositionEvent, Axis,
            InputEvent, KeyState, KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent,
        },
        renderer::{
            element::{
                solid::SolidColorRenderElement,
                surface::{render_elements_from_surface_tree, WaylandSurfaceRenderElement},
                Kind, RenderElement,
            },
            gles::GlesRenderer,
            utils::draw_render_elements,
            Color32F, Frame, Renderer,
        },
        winit::{self, WinitEvent},
    },
    input::keyboard::FilterResult,
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::{
        calloop::EventLoop,
        wayland_server::Display,
    },
    utils::{Rectangle, Transform, Logical, Physical, Point, Size, SERIAL_COUNTER},
    wayland::compositor::SurfaceAttributes,
};
use std::{sync::Arc, time::Duration};
use tracing::{error, info};

pub struct WinitData {
    pub output: Output,
}

pub fn init_winit() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("Starting CodeVerse Compositor with Winit backend");

    // Create event loop
    let mut event_loop: EventLoop<CodeVerseCompositor<WinitData>> =
        EventLoop::try_new().expect("Failed to create event loop");
    let loop_handle = event_loop.handle();

    // Create Wayland display
    let mut display: Display<CodeVerseCompositor<WinitData>> = Display::new().expect("Failed to create display");

    // Initialize winit backend
    let (mut backend, mut winit_event_loop) = winit::init::<GlesRenderer>().expect("Failed to initialize winit backend");

    // Create output
    let mode = Mode {
        size: backend.window_size(),
        refresh: 60_000, // 60 Hz
    };

    let physical_properties = PhysicalProperties {
        size: (0, 0).into(),
        subpixel: Subpixel::Unknown,
        make: "CodeVerse".into(),
        model: "Winit".into(),
        serial_number: "0".into(),
    };

    let output = Output::new("winit-0".to_string(), physical_properties);
    let display_handle = display.handle();
    output.create_global::<CodeVerseCompositor<WinitData>>(&display_handle);
    output.change_current_state(Some(mode), Some(Transform::Flipped180), None, Some((0, 0).into()));
    output.set_preferred(mode);

    // Backend data
    let backend_data = WinitData {
        output: output.clone(),
    };

    // Create compositor
    let mut compositor = CodeVerseCompositor::new(&mut display, loop_handle.clone(), backend_data);

    // Initialize workspace manager
    compositor.init_workspace_manager();

    // Add Wayland socket
    let socket = smithay::wayland::socket::ListeningSocketSource::new_auto().expect("Failed to create listening socket");
    let socket_name = socket.socket_name().to_string_lossy().into_owned();
    info!("Listening on Wayland socket: {}", socket_name);

    // Store socket name in compositor for spawning clients
    compositor.socket_name = Some(socket_name.clone());

    loop_handle
        .insert_source(socket, move |client_stream, _, state| {
            state
                .display_handle
                .insert_client(client_stream, Arc::new(ClientState::default()))
                .expect("Failed to insert client");
        })
        .expect("Failed to insert listening socket into event loop");

    // Add display to event loop
    loop_handle
        .insert_source(
            smithay::reexports::calloop::generic::Generic::new(
                display,
                smithay::reexports::calloop::Interest::READ,
                smithay::reexports::calloop::Mode::Level,
            ),
            |_, display, state| {
                // SAFETY: We don't drop the display
                unsafe {
                    display.get_mut().dispatch_clients(state).unwrap();
                }
                Ok(smithay::reexports::calloop::PostAction::Continue)
            },
        )
        .expect("Failed to insert display into event loop");

    info!("Compositor initialized, starting main loop");

    // Main event loop
    while compositor.running {
        // Dispatch events
        let result = event_loop.dispatch(Some(Duration::from_millis(16)), &mut compositor);
        if result.is_err() {
            compositor.running = false;
            break;
        }

        // Process winit events
        winit_event_loop.dispatch_new_events(|event| match event {
            WinitEvent::Resized { size, .. } => {
                info!("Window resized to {:?}", size);
                // Update output mode
                let mode = Mode {
                    size,
                    refresh: 60_000,
                };
                compositor.backend_data.output.change_current_state(
                    Some(mode),
                    None,
                    None,
                    None,
                );
            }
            WinitEvent::Input(input_event) => {
                // Handle keyboard input
                if let InputEvent::Keyboard { event } = input_event {
                    // Get keyboard from seat
                    if let Some(keyboard) = compositor.seat.get_keyboard() {
                        keyboard.input::<(), _>(
                            &mut compositor,
                            event.key_code(),
                            event.state(),
                            0.into(),
                            0,
                            |compositor_state, modifiers, keysym_handle| {
                                // Only handle key press
                                if event.state() != KeyState::Pressed {
                                    return FilterResult::Forward;
                                }

                                // Get the keysym
                                let keysym = keysym_handle.modified_sym();

                                // Try to handle as shortcut
                                if handle_keyboard_shortcut(compositor_state, keysym, *modifiers) {
                                    // Shortcut was handled, don't forward to clients
                                    FilterResult::Intercept(())
                                } else {
                                    // Not a shortcut, forward to clients
                                    FilterResult::Forward
                                }
                            },
                        );
                    }
                }

                // Handle pointer button input
                if let InputEvent::PointerButton { event } = input_event {
                    let serial = SERIAL_COUNTER.next_serial();
                    let button = event.button_code();
                    let button_state = event.state();

                    // Get current pointer location (default to 0,0 if not available)
                    let location = Point::<f64, Logical>::from((0.0, 0.0));

                    handle_pointer_button(
                        &mut compositor,
                        button,
                        button_state,
                        serial,
                        0,
                        location,
                    );
                }

                // Handle pointer motion input
                if let InputEvent::PointerMotion { ref event } = input_event {
                    // For relative motion, we'd need to track absolute position
                    // For now, skip relative motion events
                }

                // Handle pointer absolute motion (from touchpad/tablet)
                if let InputEvent::PointerMotionAbsolute { ref event } = input_event {
                    let output_size = backend.window_size();
                    // Convert from Physical to Logical
                    let logical_size = Size::<i32, Logical>::from((output_size.w, output_size.h));
                    let pos = event.position_transformed(logical_size);
                    let location = Point::<f64, Logical>::from((pos.x, pos.y));

                    handle_pointer_motion(
                        &mut compositor,
                        location,
                        0,
                    );
                }

                // Handle pointer axis (scroll wheel)
                if let InputEvent::PointerAxis { event } = input_event {
                    let horizontal = event.amount(Axis::Horizontal).unwrap_or(0.0);
                    let vertical = event.amount(Axis::Vertical).unwrap_or(0.0);

                    let frame = smithay::input::pointer::AxisFrame::new(0)
                        .source(event.source())
                        .value(Axis::Horizontal, horizontal)
                        .value(Axis::Vertical, vertical);

                    handle_pointer_axis(&mut compositor, frame);
                }
            }
            WinitEvent::CloseRequested => {
                info!("Close requested, exiting compositor");
                compositor.running = false;
            }
            _ => {}
        });

        // Calculate layout for active workspace
        let window_size = backend.window_size();
        let screen_rect = codeverse_window::Rectangle::new(
            0,
            0,
            window_size.w as u32,
            window_size.h as u32,
        );

        // Cache the screen geometry for the commit handler
        compositor.last_screen_geometry = Some(screen_rect);

        if let Some(ref mut manager) = compositor.workspace_manager {
            let gap_width = compositor.config.general.gap_width as i32;
            manager.layout_active_workspace(&mut compositor.window_tree, screen_rect, gap_width);
        }

        // Render windows
        if let Err(err) = render_output(&mut backend, &mut compositor) {
            error!("Rendering error: {}", err);
        }

        // Present
        backend.submit(None).expect("Failed to submit frame");
    }

    info!("Compositor shutting down");
    Ok(())
}

fn render_output(
    backend: &mut winit::WinitGraphicsBackend<GlesRenderer>,
    compositor: &mut CodeVerseCompositor<WinitData>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get window size for the render area
    let size = backend.window_size();
    let damage = Rectangle::from_loc_and_size((0, 0), size);

    // Get visible windows from workspace manager
    let visible_windows = if let Some(ref manager) = compositor.workspace_manager {
        manager.visible_windows(&compositor.window_tree)
    } else {
        vec![]
    };

    // Separate tiled and floating windows
    let mut tiled_windows = Vec::new();
    let mut floating_windows_data = Vec::new();

    // Collect border data for rendering
    let mut tiled_border_data: Vec<(codeverse_window::Rectangle, u32, codeverse_config::NordColor, String)> = Vec::new();
    let mut floating_border_data: Vec<(codeverse_window::Rectangle, u32, codeverse_config::NordColor, String)> = Vec::new();

    // First, collect tiled windows
    for window_id in visible_windows {
        if let Some(container) = compositor.window_tree.get(window_id) {
            if let Some(ref window_handle) = container.window {
                let geom = container.geometry;
                let location = Point::from((geom.x, geom.y));
                let surface = window_handle.wl_surface().clone();

                if !container.is_floating {
                    tiled_windows.push((surface, location));
                    // Collect border data for tiled windows
                    tiled_border_data.push((
                        geom,
                        container.border_width,
                        container.border_color,
                        format!("tiled-{:?}", window_id),
                    ));
                }
            }
        }
    }

    // Then, collect floating windows in stacking order
    for &window_id in compositor.floating_manager.get_stack() {
        if let Some(container) = compositor.window_tree.get(window_id) {
            if let Some(ref window_handle) = container.window {
                let geom = container.geometry;
                let title_bar_height = compositor.floating_manager.title_bar_height();

                // Adjust window position to account for title bar
                let window_location = Point::from((geom.x, geom.y + title_bar_height as i32));
                let surface = window_handle.wl_surface().clone();

                floating_windows_data.push((surface, window_location, geom, container.title.clone()));

                // Collect border data for floating windows (include title bar in border area)
                let bordered_geom = codeverse_window::Rectangle::new(
                    geom.x,
                    geom.y,
                    geom.width,
                    geom.height + title_bar_height,
                );
                floating_border_data.push((
                    bordered_geom,
                    container.border_width,
                    container.border_color,
                    format!("floating-{:?}", window_id),
                ));
            }
        }
    }

    // Bind the backend and get renderer + framebuffer
    let (renderer, mut framebuffer) = backend.bind()?;

    // Get Nord colors and convert to Color32F
    let bg_array = compositor.theme.background().to_f32_array();
    let bg_color = Color32F::new(bg_array[0], bg_array[1], bg_array[2], bg_array[3]);

    let title_bar_bg = compositor.theme.colors.nord1;
    let title_bar_array = title_bar_bg.to_f32_array();
    let title_bar_color = Color32F::new(title_bar_array[0], title_bar_array[1], title_bar_array[2], title_bar_array[3]);

    // Collect all render elements BEFORE starting the frame
    // Create border elements for tiled windows (only if borders are enabled)
    let borders_enabled = compositor.config.general.borders_enabled;
    let mut tiled_border_elements: Vec<BorderRenderElement> = Vec::new();
    if borders_enabled {
        for (geom, border_width, color, id) in &tiled_border_data {
            let rect: Rectangle<i32, Physical> = Rectangle::from_loc_and_size(
                (geom.x, geom.y),
                (geom.width as i32, geom.height as i32),
            );
            let borders = create_border_elements(rect, *border_width, *color, id);
            tiled_border_elements.extend(borders);
        }
    }

    // Create border elements for floating windows (only if borders are enabled)
    let mut floating_border_elements: Vec<BorderRenderElement> = Vec::new();
    if borders_enabled {
        for (geom, border_width, color, id) in &floating_border_data {
            let rect: Rectangle<i32, Physical> = Rectangle::from_loc_and_size(
                (geom.x, geom.y),
                (geom.width as i32, geom.height as i32),
            );
            let borders = create_border_elements(rect, *border_width, *color, id);
            floating_border_elements.extend(borders);
        }
    }

    // Tiled windows first
    let mut tiled_elements: Vec<WaylandSurfaceRenderElement<GlesRenderer>> = Vec::new();
    for (surface, location) in &tiled_windows {
        let elements = render_elements_from_surface_tree(
            renderer,
            surface,
            *location,
            1.0,
            1.0,
            Kind::Unspecified,
        );
        tiled_elements.extend(elements);
    }

    // Floating windows with title bars
    let title_bar_height = compositor.floating_manager.title_bar_height();
    let mut floating_title_bars = Vec::new();
    let mut floating_elements: Vec<WaylandSurfaceRenderElement<GlesRenderer>> = Vec::new();

    for (surface, window_location, geom, _title) in &floating_windows_data {
        // Store title bar rect for later drawing
        let title_bar_rect = Rectangle::from_loc_and_size(
            (geom.x, geom.y),
            (geom.width as i32, title_bar_height as i32),
        );
        floating_title_bars.push(title_bar_rect);

        // Collect window surface elements
        let elements = render_elements_from_surface_tree(
            renderer,
            surface,
            *window_location,
            1.0,
            1.0,
            Kind::Unspecified,
        );
        floating_elements.extend(elements);
    }

    // Get active workspace index for per-workspace wallpapers
    let workspace_index = compositor.workspace_manager.as_ref().map(|m| m.active_workspace_num().saturating_sub(1));

    // Load wallpaper data if configured (copy path to avoid borrow issues)
    let wallpaper_data = {
        let wallpaper_path = compositor.get_wallpaper_path(workspace_index).map(|s| s.to_string());
        if let Some(path) = wallpaper_path {
            let mode = compositor.get_wallpaper_mode(workspace_index);
            let screen_width = size.w as u32;
            let screen_height = size.h as u32;

            // Load the cached wallpaper (scales and caches if needed)
            if load_cached_wallpaper(&mut compositor.wallpaper_cache, &path, screen_width, screen_height, mode) {
                let key = make_wallpaper_key(&path, screen_width, screen_height, mode);
                compositor.wallpaper_cache.get(&key).map(|cached| {
                    (cached.data.clone(), cached.width, cached.height)
                })
            } else {
                None
            }
        } else {
            None
        }
    };

    // Import wallpaper texture BEFORE starting the frame to avoid borrow conflicts
    use smithay::backend::renderer::ImportMem;
    use smithay::backend::renderer::element::texture::{TextureBuffer, TextureRenderElement};

    let wallpaper_texture_buffer = if let Some((data, wp_width, wp_height)) = wallpaper_data {
        match renderer.import_memory(
            &data,
            smithay::backend::allocator::Fourcc::Abgr8888,
            (wp_width as i32, wp_height as i32).into(),
            false,
        ) {
            Ok(texture) => Some(TextureBuffer::from_texture(
                renderer,
                texture,
                1,
                Transform::Normal,
                None,
            )),
            Err(e) => {
                tracing::warn!("Failed to import wallpaper texture: {:?}", e);
                None
            }
        }
    } else {
        None
    };

    // Start a render frame
    let mut frame = renderer.render(&mut framebuffer, size, Transform::Flipped180)?;

    // Clear with Nord background color (this is the fallback if no wallpaper)
    frame.clear(bg_color, &[damage])?;

    // Render wallpaper texture if available
    if let Some(ref texture_buffer) = wallpaper_texture_buffer {
        let texture_element = TextureRenderElement::from_texture_buffer(
            (0.0, 0.0),
            texture_buffer,
            None,
            None,
            None,
            Kind::Unspecified,
        );
        // Draw the wallpaper
        if let Err(e) = draw_render_elements::<GlesRenderer, _, _>(&mut frame, 1.0, &[texture_element], &[damage]) {
            tracing::warn!("Failed to draw wallpaper: {:?}", e);
        }
    }

    // Draw tiled window borders
    // Note: Explicitly specify element type since SolidColorRenderElement is generic over renderer
    if let Err(e) = draw_render_elements::<GlesRenderer, _, _>(&mut frame, 1.0, &tiled_border_elements, &[damage]) {
        tracing::warn!("Failed to draw tiled border elements: {:?}", e);
    }

    // Draw tiled windows
    if let Err(e) = draw_render_elements(&mut frame, 1.0, &tiled_elements, &[damage]) {
        tracing::warn!("Failed to draw tiled window elements: {:?}", e);
    }

    // Draw floating window title bars
    for title_bar_rect in &floating_title_bars {
        if let Err(e) = frame.clear(title_bar_color, &[*title_bar_rect]) {
            tracing::warn!("Failed to draw title bar: {:?}", e);
        }
    }

    // Draw floating window borders
    if let Err(e) = draw_render_elements::<GlesRenderer, _, _>(&mut frame, 1.0, &floating_border_elements, &[damage]) {
        tracing::warn!("Failed to draw floating border elements: {:?}", e);
    }

    // Draw floating windows
    if let Err(e) = draw_render_elements(&mut frame, 1.0, &floating_elements, &[damage]) {
        tracing::warn!("Failed to draw floating window elements: {:?}", e);
    }

    // Finish the frame
    frame.finish()?;

    // Send frame callbacks to all windows
    let time = compositor.clock.now().as_millis() as u32;
    for (surface, _) in &tiled_windows {
        send_frames_surface_tree(surface, time);
    }
    for (surface, _, _, _) in &floating_windows_data {
        send_frames_surface_tree(surface, time);
    }

    Ok(())
}

/// Send frame callbacks to a surface tree
fn send_frames_surface_tree(surface: &smithay::reexports::wayland_server::protocol::wl_surface::WlSurface, time: u32) {
    use smithay::wayland::compositor::{with_surface_tree_downward, TraversalAction};

    with_surface_tree_downward(
        surface,
        (),
        |_, _, &()| TraversalAction::DoChildren(()),
        |_surf, states, &()| {
            for callback in states
                .cached_state
                .get::<SurfaceAttributes>()
                .current()
                .frame_callbacks
                .drain(..)
            {
                callback.done(time);
            }
        },
        |_, _, &()| true,
    );
}
