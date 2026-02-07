use crate::render::{ScaleMode, WallpaperCache};
use codeverse_config::{Config, NordTheme};
use codeverse_launcher::LauncherState;
use codeverse_window::{FloatingManager, NodeId, WindowTree, WindowTreeExt, WorkspaceManager};
use smithay::{
    delegate_compositor, delegate_data_device, delegate_output, delegate_seat, delegate_shm,
    delegate_xdg_shell,
    input::{keyboard::XkbConfig, Seat, SeatState},
    reexports::{
        calloop::LoopHandle,
        wayland_server::{
            backend::{ClientData, ClientId, DisconnectReason},
            Display, DisplayHandle,
        },
    },
    utils::{Clock, Logical, Monotonic, Point},
    wayland::{
        compositor::CompositorState,
        output::OutputManagerState,
        shell::xdg::{ToplevelSurface, XdgShellState},
        shm::ShmState,
    },
};
use smithay::wayland::selection::data_device::DataDeviceState;
use tracing::info;

/// Main compositor state
pub struct CodeVerseCompositor<BackendData: 'static> {
    /// Wayland display handle
    pub display_handle: DisplayHandle,

    /// Event loop handle
    pub loop_handle: LoopHandle<'static, Self>,

    /// Smithay compositor state (handles wl_surface, wl_region, etc.)
    pub compositor_state: CompositorState,

    /// XDG shell state (handles windows, popups)
    pub xdg_shell_state: XdgShellState,

    /// Seat state (input devices)
    pub seat_state: SeatState<Self>,

    /// SHM (shared memory) state
    pub shm_state: ShmState,

    /// Data device state (clipboard, drag-and-drop)
    pub data_device_state: DataDeviceState,

    /// Output manager state (displays)
    pub output_manager_state: OutputManagerState,

    /// Seat for input
    pub seat: Seat<Self>,

    /// Window tree for tiling
    pub window_tree: WindowTree,

    /// Workspace manager (initialized after output is created)
    pub workspace_manager: Option<WorkspaceManager>,

    /// Floating window manager
    pub floating_manager: FloatingManager,

    /// Output node ID in the tree
    pub output_node: Option<NodeId>,

    /// Configuration
    pub config: Config,

    /// Nord theme
    pub theme: NordTheme,

    /// Monotonic clock
    pub clock: Clock<Monotonic>,

    /// Should the compositor exit?
    pub running: bool,

    /// Wayland socket name for spawning clients
    pub socket_name: Option<String>,

    /// Launcher state
    pub launcher: Option<LauncherState>,

    /// Is launcher currently active?
    pub launcher_active: bool,

    /// Wallpaper cache for storing loaded and scaled textures
    pub wallpaper_cache: WallpaperCache,

    /// Cached screen geometry (updated during rendering, used by commit handler)
    pub last_screen_geometry: Option<codeverse_window::Rectangle>,

    /// Current pointer location (tracked for DRM/bare-metal backends)
    pub pointer_location: Point<f64, Logical>,

    /// Backend-specific data
    pub backend_data: BackendData,
}

impl<BackendData> CodeVerseCompositor<BackendData> {
    pub fn new(
        display: &mut Display<Self>,
        loop_handle: LoopHandle<'static, Self>,
        backend_data: BackendData,
    ) -> Self {
        let display_handle = display.handle();

        // Initialize Smithay states
        let compositor_state = CompositorState::new::<Self>(&display_handle);
        let xdg_shell_state = XdgShellState::new::<Self>(&display_handle);
        let shm_state = ShmState::new::<Self>(&display_handle, vec![]);
        let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(&display_handle);
        let mut seat_state = SeatState::new();
        let data_device_state = DataDeviceState::new::<Self>(&display_handle);

        // Create seat
        let mut seat = seat_state.new_wl_seat(&display_handle, "seat-0");

        // Add keyboard capability
        seat.add_keyboard(XkbConfig::default(), 200, 25)
            .expect("Failed to add keyboard to seat");

        // Add pointer capability
        seat.add_pointer();

        // Load configuration
        let config = Config::load().unwrap_or_else(|e| {
            tracing::warn!("Failed to load config: {}, using defaults", e);
            Config::default()
        });

        let window_tree = WindowTree::new();
        let workspace_manager = None; // Will be initialized when output is created
        let floating_manager = FloatingManager::new();
        let output_node = None;
        let theme = config.get_theme();
        let clock = Clock::new();

        Self {
            display_handle,
            loop_handle,
            compositor_state,
            xdg_shell_state,
            seat_state,
            shm_state,
            data_device_state,
            output_manager_state,
            seat,
            window_tree,
            workspace_manager,
            floating_manager,
            output_node,
            config,
            theme,
            clock,
            running: true,
            socket_name: None,
            launcher: None, // Initialized lazily on first use
            launcher_active: false,
            wallpaper_cache: WallpaperCache::new(),
            last_screen_geometry: None,
            pointer_location: (0.0, 0.0).into(),
            backend_data,
        }
    }

    /// Initialize workspace manager for an output
    pub fn init_workspace_manager(&mut self) {
        use codeverse_window::{Container, ContainerType};

        // Create output node in tree
        let output = Container::new(NodeId::default(), ContainerType::Output);
        let output_id = self.window_tree.insert(output);

        // Create workspace manager
        let workspace_manager = WorkspaceManager::new(&mut self.window_tree, output_id);

        self.output_node = Some(output_id);
        self.workspace_manager = Some(workspace_manager);

        info!("Workspace manager initialized with output {:?}", output_id);
    }

    /// Handle a new toplevel window
    pub fn handle_new_toplevel(&mut self, toplevel: ToplevelSurface) {
        info!("New toplevel window created");

        // Insert window into tree
        if let Some(ref mut workspace_manager) = self.workspace_manager {
            if let Some(workspace_id) = workspace_manager.active_workspace() {
                match self.window_tree.insert_window(toplevel.clone(), workspace_id) {
                    Ok(window_id) => {
                        info!("Window inserted into tree with id {:?}", window_id);

                        // Set border properties from config
                        if let Some(container) = self.window_tree.get_mut(window_id) {
                            container.border_width = self.config.general.border_width;
                            // New windows start unfocused
                            container.border_color = self.theme.unfocused_border();
                        }

                        // Update border colors for all windows (the new window may be focused)
                        self.update_window_border_colors();

                        // Send initial configure with a default size
                        toplevel.with_pending_state(|state| {
                            state.size = Some((800, 600).into());
                        });
                        toplevel.send_configure();

                        // Track the initial configured size
                        if let Some(container) = self.window_tree.get_mut(window_id) {
                            container.last_configured_size = Some((800, 600));
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to insert window into tree: {}", e);
                    }
                }
            } else {
                tracing::error!("No active workspace found");
            }
        } else {
            tracing::warn!("Workspace manager not initialized, window not tiled");

            // Fallback: just configure the window
            toplevel.with_pending_state(|state| {
                state.size = Some((800, 600).into());
            });
            toplevel.send_configure();
        }
    }

    /// Handle window close
    pub fn handle_toplevel_closed(&mut self, toplevel: &ToplevelSurface) {
        info!("Toplevel window closed");

        // Find and remove window from tree
        if let Some(window_id) = self.window_tree.find_window_by_handle(toplevel) {
            match self.window_tree.remove_window(window_id) {
                Ok(()) => {
                    info!("Window {:?} removed from tree", window_id);
                }
                Err(e) => {
                    tracing::error!("Failed to remove window from tree: {}", e);
                }
            }
        } else {
            tracing::warn!("Could not find window in tree to remove");
        }
    }

    /// Toggle the launcher on/off
    pub fn toggle_launcher(&mut self) {
        self.launcher_active = !self.launcher_active;

        if self.launcher_active {
            // Initialize launcher if not yet created
            if self.launcher.is_none() {
                info!("Initializing launcher (first time)...");
                self.launcher = Some(LauncherState::new());
            }

            // Reset launcher state when opening
            if let Some(ref mut launcher) = self.launcher {
                launcher.reset();
            }

            info!("Launcher opened");
        } else {
            info!("Launcher closed");
        }
    }

    /// Launch the selected app from the launcher
    pub fn launch_selected_app(&mut self) -> Result<(), String> {
        if !self.launcher_active {
            return Err("Launcher is not active".to_string());
        }

        let launcher = self.launcher.as_ref().ok_or("Launcher not initialized")?;
        let app = launcher.selected_app().ok_or("No app selected")?;

        info!("Launching app: {} ({})", app.name, app.exec);

        // Get the command to execute
        let command = app.get_command();

        // Spawn the application
        use std::process::Command;
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(&command);

        // Set WAYLAND_DISPLAY to connect to our compositor
        if let Some(ref socket) = self.socket_name {
            cmd.env("WAYLAND_DISPLAY", socket);
        }

        match cmd.spawn() {
            Ok(child) => {
                info!("Launched {} (PID: {})", app.name, child.id());
                // Close launcher after successful launch
                self.launcher_active = false;
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to launch {}: {}", app.name, e);
                Err(format!("Failed to launch: {}", e))
            }
        }
    }

    /// Reload configuration from disk
    pub fn reload_config(&mut self) {
        info!("Reloading configuration...");

        match Config::load() {
            Ok(new_config) => {
                self.config = new_config;
                self.theme = self.config.get_theme();
                info!("Configuration reloaded successfully");

                // Update border colors after config reload
                self.update_window_border_colors();

                // Clear wallpaper cache to force reload on next render
                self.wallpaper_cache.clear();
                info!("Wallpaper cache cleared");
            }
            Err(e) => {
                tracing::error!("Failed to reload config: {}", e);
            }
        }
    }

    /// Get the wallpaper path for the current workspace
    pub fn get_wallpaper_path(&self, workspace_index: Option<usize>) -> Option<&str> {
        // Check for per-workspace wallpaper first
        if let Some(index) = workspace_index {
            if let Some(ws_wallpaper) = self
                .config
                .wallpaper
                .per_workspace
                .iter()
                .find(|ws| ws.workspace == index + 1)
            {
                return Some(&ws_wallpaper.path);
            }
        }

        // Fall back to global wallpaper
        self.config.wallpaper.path.as_deref()
    }

    /// Get the wallpaper scale mode for the current workspace
    pub fn get_wallpaper_mode(&self, workspace_index: Option<usize>) -> ScaleMode {
        // Check for per-workspace wallpaper mode first
        if let Some(index) = workspace_index {
            if let Some(ws_wallpaper) = self
                .config
                .wallpaper
                .per_workspace
                .iter()
                .find(|ws| ws.workspace == index + 1)
            {
                if let Some(ref mode) = ws_wallpaper.mode {
                    return ScaleMode::from_str(mode);
                }
            }
        }

        // Fall back to global wallpaper mode
        ScaleMode::from_str(&self.config.wallpaper.mode)
    }

    /// Send configure events to windows whose layout size has changed
    pub fn send_pending_configures(&mut self) {
        let window_ids: Vec<NodeId> = self.window_tree.find_windows();

        for window_id in window_ids {
            // Read geometry and check if configure is needed
            let configure_info = if let Some(container) = self.window_tree.get(window_id) {
                let geom = container.geometry;
                let new_size = (geom.width, geom.height);

                // Skip if size is zero (not yet laid out)
                if new_size.0 == 0 || new_size.1 == 0 {
                    continue;
                }

                // Skip if size hasn't changed
                if container.last_configured_size == Some(new_size) {
                    continue;
                }

                // Need the ToplevelSurface handle
                container.window.clone().map(|w| (w, new_size))
            } else {
                continue;
            };

            if let Some((toplevel, new_size)) = configure_info {
                // Send configure with the layout-assigned size
                toplevel.with_pending_state(|state| {
                    state.size = Some((new_size.0 as i32, new_size.1 as i32).into());
                });
                toplevel.send_configure();

                // Update last_configured_size
                if let Some(container) = self.window_tree.get_mut(window_id) {
                    container.last_configured_size = Some(new_size);
                }
            }
        }
    }

    /// Find the Wayland surface under a given point (for seat pointer focus).
    /// Returns the focus target and surface-local coordinates.
    /// Checks floating windows first (top of stack), then tiled windows.
    pub fn surface_under(&self, pos: Point<f64, Logical>) -> Option<(crate::focus::PointerFocusTarget, Point<f64, Logical>)> {
        let x = pos.x as i32;
        let y = pos.y as i32;

        // Check floating windows first (rendered on top), reverse stacking order
        for &window_id in self.floating_manager.get_stack().iter().rev() {
            if let Some(container) = self.window_tree.get(window_id) {
                if container.is_floating {
                    if let Some(ref toplevel) = container.window {
                        let geom = container.geometry;
                        let title_bar_height = self.floating_manager.title_bar_height() as i32;
                        let surface_y = geom.y + title_bar_height;

                        // Check if point is within the surface area (below title bar)
                        if x >= geom.x && x < geom.x + geom.width as i32
                            && y >= surface_y && y < surface_y + geom.height as i32
                        {
                            let surface = toplevel.wl_surface().clone();
                            let surface_local = Point::from((
                                pos.x - geom.x as f64,
                                pos.y - surface_y as f64,
                            ));
                            return Some((crate::focus::PointerFocusTarget::Surface(surface), surface_local));
                        }
                    }
                }
            }
        }

        // Check tiled windows
        if let Some(ref manager) = self.workspace_manager {
            let visible = manager.visible_windows(&self.window_tree);
            for window_id in visible {
                if let Some(container) = self.window_tree.get(window_id) {
                    if !container.is_floating {
                        let geom = container.geometry;
                        if x >= geom.x && x < geom.x + geom.width as i32
                            && y >= geom.y && y < geom.y + geom.height as i32
                        {
                            if let Some(ref toplevel) = container.window {
                                let surface = toplevel.wl_surface().clone();
                                let surface_local = Point::from((
                                    pos.x - geom.x as f64,
                                    pos.y - geom.y as f64,
                                ));
                                return Some((crate::focus::PointerFocusTarget::Surface(surface), surface_local));
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Find the window (NodeId) under a given point.
    /// Checks floating windows first (including title bar area), then tiled.
    pub fn window_under(&self, pos: Point<f64, Logical>) -> Option<NodeId> {
        let x = pos.x as i32;
        let y = pos.y as i32;

        // Check floating windows first (including title bar area)
        for &window_id in self.floating_manager.get_stack().iter().rev() {
            if let Some(container) = self.window_tree.get(window_id) {
                if container.is_floating {
                    let geom = container.geometry;
                    let title_bar_height = self.floating_manager.title_bar_height() as i32;
                    let total_height = geom.height as i32 + title_bar_height;
                    if x >= geom.x && x < geom.x + geom.width as i32
                        && y >= geom.y && y < geom.y + total_height
                    {
                        return Some(window_id);
                    }
                }
            }
        }

        // Check tiled windows
        if let Some(ref manager) = self.workspace_manager {
            let visible = manager.visible_windows(&self.window_tree);
            for window_id in visible {
                if let Some(container) = self.window_tree.get(window_id) {
                    if !container.is_floating {
                        let geom = container.geometry;
                        if x >= geom.x && x < geom.x + geom.width as i32
                            && y >= geom.y && y < geom.y + geom.height as i32
                        {
                            return Some(window_id);
                        }
                    }
                }
            }
        }

        None
    }

    /// Update border colors for all windows based on focus state
    pub fn update_window_border_colors(&mut self) {
        let focused_color = self.theme.focused_border();
        let unfocused_color = self.theme.unfocused_border();
        let focused_id = self.window_tree.focused();

        // Get all window IDs first to avoid borrow conflicts
        let window_ids: Vec<NodeId> = self.window_tree.find_windows();

        for window_id in window_ids {
            if let Some(container) = self.window_tree.get_mut(window_id) {
                let is_focused = Some(window_id) == focused_id;
                container.border_color = if is_focused {
                    focused_color
                } else {
                    unfocused_color
                };
                container.border_width = self.config.general.border_width;
            }
        }
    }
}

// Smithay delegate implementations
delegate_compositor!(@<BackendData: 'static> CodeVerseCompositor<BackendData>);
delegate_xdg_shell!(@<BackendData: 'static> CodeVerseCompositor<BackendData>);
delegate_shm!(@<BackendData: 'static> CodeVerseCompositor<BackendData>);
delegate_seat!(@<BackendData: 'static> CodeVerseCompositor<BackendData>);
delegate_data_device!(@<BackendData: 'static> CodeVerseCompositor<BackendData>);
delegate_output!(@<BackendData: 'static> CodeVerseCompositor<BackendData>);

/// Client data for Wayland clients
#[derive(Default)]
pub struct ClientState {
    pub compositor_state: smithay::wayland::compositor::CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {
        info!("Client initialized");
    }

    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {
        info!("Client disconnected");
    }
}
