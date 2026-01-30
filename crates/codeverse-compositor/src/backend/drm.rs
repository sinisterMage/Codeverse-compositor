use crate::compositor::{ClientState, CodeVerseCompositor};
use smithay::{
    backend::{
        allocator::{
            gbm::{GbmAllocator, GbmBufferFlags, GbmDevice},
            Fourcc,
        },
        drm::{
            exporter::gbm::GbmFramebufferExporter,
            output::{DrmOutput, DrmOutputManager},
            DrmDevice, DrmDeviceFd, DrmEvent, DrmNode, NodeType,
        },
        egl::{EGLContext, EGLDevice, EGLDisplay},
        input::InputEvent,
        libinput::{LibinputInputBackend, LibinputSessionInterface},
        renderer::{
            damage::OutputDamageTracker,
            element::{
                surface::{render_elements_from_surface_tree, WaylandSurfaceRenderElement},
                Kind,
            },
            gles::GlesRenderer,
            multigpu::{gbm::GbmGlesBackend, GpuManager, MultiRenderer, MultiTexture},
            Color32F,
        },
        session::{
            libseat::LibSeatSession,
            Event as SessionEvent, Session,
        },
        udev::{all_gpus, primary_gpu, UdevBackend, UdevEvent},
    },
    output::{Mode as WlMode, Output, PhysicalProperties, Subpixel},
    reexports::{
        calloop::{EventLoop, RegistrationToken},
        drm::control::{connector, crtc, ModeTypeFlags},
        input::Libinput,
        rustix::fs::OFlags,
        wayland_server::{Display, DisplayHandle},
    },
    utils::{DeviceFd, Point},
    wayland::socket::ListeningSocketSource,
};
use smithay_drm_extras::drm_scanner::{DrmScanEvent, DrmScanner};
use std::{
    collections::HashMap,
    path::Path,
    sync::Arc,
    time::Duration,
};
use tracing::{debug, error, info, warn};

// Supported formats for rendering (from anvil)
const SUPPORTED_FORMATS: &[Fourcc] = &[
    Fourcc::Abgr2101010,
    Fourcc::Argb2101010,
    Fourcc::Abgr8888,
    Fourcc::Argb8888,
];

pub struct DrmBackendData {
    pub session: LibSeatSession,
    dh: DisplayHandle,
    primary_gpu: DrmNode,
    gpus: GpuManager<GbmGlesBackend<GlesRenderer, DrmDeviceFd>>,
    backends: HashMap<DrmNode, BackendData>,
}

// User data type for DrmOutput (passed to queue_frame, returned on vblank)
type FrameUserData = ();

// Type alias for the DRM output manager
type DrmOutputMgr = DrmOutputManager<GbmAllocator<DrmDeviceFd>, GbmFramebufferExporter<DrmDeviceFd>, FrameUserData, DrmDeviceFd>;
type DrmOutputType = DrmOutput<GbmAllocator<DrmDeviceFd>, GbmFramebufferExporter<DrmDeviceFd>, FrameUserData, DrmDeviceFd>;

struct BackendData {
    _registration_token: RegistrationToken,
    drm_output_manager: DrmOutputMgr,
    drm_scanner: DrmScanner,
    render_node: Option<DrmNode>,
    surfaces: HashMap<crtc::Handle, SurfaceData>,
}

struct SurfaceData {
    output: Output,
    drm_output: DrmOutputType,
    damage_tracker: OutputDamageTracker,
}

pub fn init_drm() -> Result<(), Box<dyn std::error::Error>> {
    let mut event_loop = EventLoop::try_new()?;
    let mut display = Display::new()?;
    let display_handle = display.handle();

    // Initialize session (from anvil)
    let (session, notifier) = LibSeatSession::new()?;
    info!("Session created on seat: {}", session.seat());

    // Determine primary GPU (from anvil)
    let primary_gpu = primary_gpu(session.seat())?
        .and_then(|x| DrmNode::from_path(x).ok()?.node_with_type(NodeType::Render)?.ok())
        .unwrap_or_else(|| {
            all_gpus(session.seat())
                .unwrap()
                .into_iter()
                .find_map(|x| DrmNode::from_path(x).ok())
                .expect("No GPU found!")
        });
    info!("Using {} as primary GPU", primary_gpu);

    // Create GPU manager (from anvil)
    let gpus = GpuManager::new(GbmGlesBackend::with_factory(|display| {
        let context = EGLContext::new(display)?;
        let capabilities = unsafe { GlesRenderer::supported_capabilities(&context)? };
        Ok(unsafe { GlesRenderer::with_capabilities(context, capabilities)? })
    }))?;

    let data = DrmBackendData {
        dh: display_handle.clone(),
        session,
        primary_gpu,
        gpus,
        backends: HashMap::new(),
    };

    let mut compositor = CodeVerseCompositor::new(&mut display, event_loop.handle(), data);

    // Initialize workspace manager (required for keybindings to work)
    compositor.init_workspace_manager();

    // Initialize udev backend
    let udev_backend = UdevBackend::new(&compositor.backend_data.session.seat())?;

    // Initialize libinput (from anvil)
    let mut libinput_context = Libinput::new_with_udev::<LibinputSessionInterface<LibSeatSession>>(
        compositor.backend_data.session.clone().into(),
    );
    libinput_context
        .udev_assign_seat(&compositor.backend_data.session.seat())
        .map_err(|()| "Failed to assign seat to libinput")?;
    let libinput_backend = LibinputInputBackend::new(libinput_context.clone());

    // Insert libinput event source (simplified from anvil)
    event_loop
        .handle()
        .insert_source(libinput_backend, move |event, _, compositor| {
            compositor.handle_input_event(event);
        })?;

    // Insert session event source (from anvil)
    event_loop
        .handle()
        .insert_source(notifier, move |event, &mut (), compositor| match event {
            SessionEvent::PauseSession => {
                libinput_context.suspend();
                info!("Session paused");
                for backend in compositor.backend_data.backends.values_mut() {
                    backend.drm_output_manager.pause();
                }
            }
            SessionEvent::ActivateSession => {
                info!("Session activated");
                if let Err(err) = libinput_context.resume() {
                    error!("Failed to resume libinput: {:?}", err);
                }
                for backend in compositor.backend_data.backends.values_mut() {
                    backend
                        .drm_output_manager
                        .lock()
                        .activate(false)
                        .expect("Failed to activate DRM backend");
                }
            }
        })?;

    // Process existing DRM devices
    for (device_id, path) in udev_backend.device_list() {
        if let Ok(node) = DrmNode::from_dev_id(device_id) {
            if let Err(e) = compositor.device_added(node, &path) {
                error!("Failed to add device {:?}: {}", path, e);
            }
        }
    }

    // Insert udev event source (from anvil)
    event_loop
        .handle()
        .insert_source(udev_backend, move |event, _, compositor| match event {
            UdevEvent::Added { device_id, path } => {
                if let Ok(node) = DrmNode::from_dev_id(device_id) {
                    if let Err(e) = compositor.device_added(node, &path) {
                        error!("Failed to add device {:?}: {}", path, e);
                    }
                }
            }
            UdevEvent::Changed { device_id } => {
                if let Ok(node) = DrmNode::from_dev_id(device_id) {
                    compositor.device_changed(node);
                }
            }
            UdevEvent::Removed { device_id } => {
                if let Ok(node) = DrmNode::from_dev_id(device_id) {
                    compositor.device_removed(node);
                }
            }
        })?;

    // Create Wayland socket
    let socket_source = ListeningSocketSource::new_auto()?;
    let socket_name = socket_source.socket_name().to_string_lossy().into_owned();
    info!("Wayland socket: {}", socket_name);

    // Store socket name so F12 can spawn terminals connected to our compositor
    compositor.socket_name = Some(socket_name);

    event_loop
        .handle()
        .insert_source(socket_source, move |client_stream, _, compositor| {
            if let Err(err) = compositor
                .display_handle
                .insert_client(client_stream, Arc::new(ClientState::default()))
            {
                error!("Error adding wayland client: {}", err);
            }
        })?;

    // Main event loop
    info!("Starting CodeVerse Compositor with DRM backend");
    loop {
        // Dispatch pending events
        event_loop.dispatch(Some(Duration::from_millis(16)), &mut compositor)?;

        // Render all outputs
        compositor.render_all_outputs();

        // Flush Wayland clients
        compositor.display_handle.flush_clients().ok();
    }
}

// Implement DRM-specific methods for CodeVerseCompositor
impl CodeVerseCompositor<DrmBackendData> {
    // Adapted from anvil's device_added
    fn device_added(&mut self, node: DrmNode, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        info!("Adding DRM device: {:?}", path);

        // Open the device (from anvil)
        let fd = self
            .backend_data
            .session
            .open(
                path,
                OFlags::RDWR | OFlags::CLOEXEC | OFlags::NOCTTY | OFlags::NONBLOCK,
            )?;

        let fd = DrmDeviceFd::new(DeviceFd::from(fd));

        // Create DRM device (from anvil)
        let (drm, notifier) = DrmDevice::new(fd.clone(), true)?;
        let gbm = GbmDevice::new(fd)?;

        // Register DRM event handler (from anvil)
        let node_clone = node;
        let registration_token = self.loop_handle.insert_source(
            notifier,
            move |event, _metadata, compositor| match event {
                DrmEvent::VBlank(crtc) => {
                    debug!("VBlank on {:?} crtc {:?}", node_clone, crtc);
                    compositor.on_vblank(node_clone, crtc);
                }
                DrmEvent::Error(error) => {
                    error!("DRM error: {:?}", error);
                }
            },
        )?;

        // Initialize GPU (from anvil)
        let display = unsafe { EGLDisplay::new(gbm.clone())? };
        let egl_device = EGLDevice::device_for_display(&display)?;

        let render_node = if egl_device.is_software() {
            warn!("Device is software renderer");
            None
        } else {
            egl_device.try_get_render_node().ok().flatten().or(Some(node))
        };

        if let Some(render_node) = render_node {
            self
                .backend_data
                .gpus
                .as_mut()
                .add_node(render_node, gbm.clone())?;
        }

        // Create allocator (from anvil)
        let allocator = GbmAllocator::new(gbm.clone(), GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT);

        // Create framebuffer exporter (from anvil)
        let framebuffer_exporter = GbmFramebufferExporter::new(gbm.clone(), render_node.into());

        // Get render formats (from anvil)
        let mut renderer = self
            .backend_data
            .gpus
            .single_renderer(&render_node.unwrap_or(self.backend_data.primary_gpu))?;
        let render_formats = renderer
            .as_mut()
            .egl_context()
            .dmabuf_render_formats()
            .clone();

        // Create DRM output manager (from anvil)
        let drm_output_manager = DrmOutputManager::new(
            drm,
            allocator,
            framebuffer_exporter,
            Some(gbm),
            SUPPORTED_FORMATS.iter().copied(),
            render_formats,
        );

        let backend_data = BackendData {
            _registration_token: registration_token,
            drm_output_manager,
            drm_scanner: DrmScanner::new(),
            render_node,
            surfaces: HashMap::new(),
        };

        self.backend_data.backends.insert(node, backend_data);

        self.device_changed(node);

        Ok(())
    }

    // Adapted from anvil's device_changed
    fn device_changed(&mut self, node: DrmNode) {
        let Some(backend) = self.backend_data.backends.get_mut(&node) else {
            return;
        };

        let drm_device = backend.drm_output_manager.device();

        // Scan connectors (from anvil)
        let scan_result = match backend.drm_scanner.scan_connectors(drm_device) {
            Ok(result) => result,
            Err(err) => {
                warn!("Failed to scan connectors: {:?}", err);
                return;
            }
        };

        for event in scan_result {
            match event {
                DrmScanEvent::Connected {
                    connector,
                    crtc: Some(crtc),
                } => {
                    self.connector_connected(node, connector, crtc);
                }
                DrmScanEvent::Disconnected {
                    connector,
                    crtc: Some(crtc),
                } => {
                    self.connector_disconnected(node, connector, crtc);
                }
                _ => {}
            }
        }
    }

    // Adapted from anvil's connector_connected
    fn connector_connected(&mut self, node: DrmNode, connector: connector::Info, crtc: crtc::Handle) {
        let Some(backend) = self.backend_data.backends.get_mut(&node) else {
            return;
        };

        let render_node = backend.render_node.unwrap_or(self.backend_data.primary_gpu);
        let mut renderer = match self.backend_data.gpus.single_renderer(&render_node) {
            Ok(r) => r,
            Err(e) => {
                error!("Failed to get renderer: {:?}", e);
                return;
            }
        };

    let output_name = format!("{}-{}", connector.interface().as_str(), connector.interface_id());
    info!("Connector {} connected on crtc {:?}", output_name, crtc);

    // Get the preferred mode (from anvil)
    let mode = connector
        .modes()
        .iter()
        .find(|mode| mode.mode_type().contains(ModeTypeFlags::PREFERRED))
        .or_else(|| connector.modes().first())
        .copied()
        .unwrap_or_else(|| {
            panic!("No mode available for connector {}", output_name);
        });

    let drm_mode = mode;
    let wl_mode = WlMode::from(mode);

    // Create Smithay output (from anvil)
    let output = Output::new(
        output_name.clone(),
        PhysicalProperties {
            size: (
                connector.size().unwrap_or((0, 0)).0 as i32,
                connector.size().unwrap_or((0, 0)).1 as i32,
            )
                .into(),
            subpixel: Subpixel::Unknown,
            make: "Unknown".into(),
            model: output_name.clone(),
            serial_number: "0".into(),
        },
    );

    output.change_current_state(Some(wl_mode), None, None, Some((0, 0).into()));
    output.set_preferred(wl_mode);

    // Initialize output with DRM compositor (from anvil pattern)
    let drm_device = backend.drm_output_manager.device();

    // Get planes for the crtc
    let planes = match drm_device.planes(&crtc) {
        Ok(planes) => Some(planes),
        Err(err) => {
            warn!("Failed to query crtc planes: {}", err);
            None
        }
    };

    // Initialize the DRM output (from anvil)
    // We use MultiRenderer type
    type Renderer<'a> = MultiRenderer<'a, 'a, GbmGlesBackend<GlesRenderer, DrmDeviceFd>, GbmGlesBackend<GlesRenderer, DrmDeviceFd>>;
    type RenderElement<'a> = WaylandSurfaceRenderElement<Renderer<'a>>;

    // Empty render elements for initialization
    use smithay::backend::drm::output::DrmOutputRenderElements;
    let init_elements: DrmOutputRenderElements<Renderer<'_>, RenderElement<'_>> = DrmOutputRenderElements::default();

    let drm_output = match backend
        .drm_output_manager
        .lock()
        .initialize_output::<Renderer<'_>, RenderElement<'_>>(
            crtc,
            drm_mode,
            &[connector.handle()],
            &output,
            planes,
            &mut renderer,
            &init_elements,
        ) {
        Ok(output) => output,
        Err(err) => {
            error!("Failed to initialize drm output: {}", err);
            return;
        }
    };

    // Create damage tracker for the output
    let damage_tracker = OutputDamageTracker::from_output(&output);

    backend.surfaces.insert(
        crtc,
        SurfaceData {
            output: output.clone(),
            drm_output,
            damage_tracker,
        },
    );

        info!("Output {} configured with mode {:?}", output_name, wl_mode);
    }

    // Adapted from anvil's connector_disconnected
    fn connector_disconnected(&mut self, node: DrmNode, _connector: connector::Info, crtc: crtc::Handle) {
        let Some(backend) = self.backend_data.backends.get_mut(&node) else {
            return;
        };

        if let Some(_surface_data) = backend.surfaces.remove(&crtc) {
            info!("Connector on crtc {:?} disconnected", crtc);
        }
    }

    // Adapted from anvil's device_removed
    fn device_removed(&mut self, node: DrmNode) {
        if let Some(_backend) = self.backend_data.backends.remove(&node) {
            info!("DRM device {:?} removed", node);
        }
    }

    /// Handle input events from libinput
    fn handle_input_event(&mut self, event: InputEvent<LibinputInputBackend>) {
        use crate::input::handle_keyboard_shortcut;
        use smithay::backend::input::{Event, KeyState, KeyboardKeyEvent};
        use smithay::input::keyboard::FilterResult;

        match event {
            InputEvent::Keyboard { event } => {
                let serial = smithay::utils::SERIAL_COUNTER.next_serial();
                let time = Event::time_msec(&event);
                let key_code = event.key_code();
                let state = event.state();

                debug!("Keyboard event: key={:?} state={:?}", key_code, state);

                // Process through the seat keyboard
                let keyboard = self.seat.get_keyboard().unwrap();
                keyboard.input::<(), _>(
                    self,
                    key_code,
                    state,
                    serial,
                    time,
                    |compositor, modifiers, keysym_handle| {
                        // Only handle key press events for shortcuts
                        if state != KeyState::Pressed {
                            return FilterResult::Forward;
                        }

                        let keysym = keysym_handle.modified_sym();

                        // Try to handle as compositor shortcut
                        if handle_keyboard_shortcut(compositor, keysym, *modifiers) {
                            info!("Shortcut handled: {:?}", keysym);
                            FilterResult::Intercept(())
                        } else {
                            // Forward to focused client
                            FilterResult::Forward
                        }
                    },
                );
            }
            InputEvent::PointerMotion { event } => {
                debug!("Pointer motion event");
                // TODO: Handle pointer motion properly
            }
            InputEvent::PointerMotionAbsolute { event } => {
                debug!("Pointer absolute motion event");
                // TODO: Handle absolute pointer motion
            }
            InputEvent::PointerButton { event } => {
                debug!("Pointer button event");
                // TODO: Handle pointer button properly
            }
            InputEvent::PointerAxis { event } => {
                debug!("Pointer axis event");
                // TODO: Handle pointer scroll
            }
            _ => {
                // Other events (touch, tablet, etc.)
            }
        }
    }

    /// Handle VBlank event - called when a frame has been displayed
    fn on_vblank(&mut self, node: DrmNode, crtc: crtc::Handle) {
        let Some(backend) = self.backend_data.backends.get_mut(&node) else {
            return;
        };

        let Some(surface_data) = backend.surfaces.get_mut(&crtc) else {
            return;
        };

        // Notify the DRM output that the frame has been submitted
        if let Err(e) = surface_data.drm_output.frame_submitted() {
            warn!("Failed to mark frame as submitted: {:?}", e);
        }
    }

    /// Render all outputs
    fn render_all_outputs(&mut self) {
        // Collect what we need to render
        let nodes: Vec<DrmNode> = self.backend_data.backends.keys().copied().collect();

        for node in nodes {
            self.render_node_outputs(node);
        }
    }

    /// Render all outputs for a specific DRM node
    fn render_node_outputs(&mut self, node: DrmNode) {
        let Some(backend) = self.backend_data.backends.get_mut(&node) else {
            return;
        };

        let render_node = backend.render_node.unwrap_or(self.backend_data.primary_gpu);

        // Collect crtcs to render
        let crtcs: Vec<crtc::Handle> = backend.surfaces.keys().copied().collect();

        for crtc in crtcs {
            if let Err(e) = self.render_surface(node, crtc, render_node) {
                warn!("Failed to render surface on {:?}: {}", crtc, e);
            }
        }
    }

    /// Render a single surface/output
    fn render_surface(
        &mut self,
        node: DrmNode,
        crtc: crtc::Handle,
        render_node: DrmNode,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use smithay::backend::drm::compositor::FrameFlags;

        // Renderer type alias for this function
        type Renderer<'a> = MultiRenderer<'a, 'a, GbmGlesBackend<GlesRenderer, DrmDeviceFd>, GbmGlesBackend<GlesRenderer, DrmDeviceFd>>;
        type RenderElement<'a> = WaylandSurfaceRenderElement<Renderer<'a>>;

        // Get renderer for this GPU
        let mut renderer = self.backend_data.gpus.single_renderer(&render_node)?;

        // Get screen geometry from the output for layout calculation
        let screen_geometry = {
            let backend = self.backend_data.backends.get(&node).ok_or("Backend not found for geometry")?;
            let surface_data = backend.surfaces.get(&crtc).ok_or("Surface not found for geometry")?;
            let mode = surface_data.output.current_mode().ok_or("No output mode")?;
            codeverse_window::Rectangle {
                x: 0,
                y: 0,
                width: mode.size.w as u32,
                height: mode.size.h as u32,
            }
        };

        // Calculate layout before rendering to ensure windows have proper geometries
        if let Some(ref mut manager) = self.workspace_manager {
            manager.layout_active_workspace(&mut self.window_tree, screen_geometry);
        }

        // Collect visible windows and their surfaces
        let visible_windows = if let Some(ref manager) = self.workspace_manager {
            let windows = manager.visible_windows(&self.window_tree);
            debug!("WorkspaceManager found {} visible windows", windows.len());
            windows
        } else {
            debug!("No WorkspaceManager, no visible windows");
            vec![]
        };

        // Collect window surfaces with their locations
        let mut window_surfaces = Vec::new();
        for window_id in &visible_windows {
            if let Some(container) = self.window_tree.get(*window_id) {
                debug!("Window {:?} found in tree, has_window={}", window_id, container.window.is_some());
                if let Some(ref window_handle) = container.window {
                    let geom = container.geometry;
                    let location = Point::from((geom.x, geom.y));
                    let surface = window_handle.wl_surface().clone();
                    debug!("Window {:?} at location {:?}, geom: {:?}", window_id, location, geom);
                    window_surfaces.push((surface, location));
                }
            } else {
                debug!("Window {:?} NOT found in tree", window_id);
            }
        }

        debug!("Collected {} window surfaces for rendering", window_surfaces.len());

        // Create render elements from window surfaces
        let mut render_elements: Vec<RenderElement<'_>> = Vec::new();
        for (surface, location) in &window_surfaces {
            let elements = render_elements_from_surface_tree(
                &mut renderer,
                surface,
                *location,
                1.0,
                1.0,
                Kind::Unspecified,
            );
            debug!("Created {} render elements for surface at {:?}", elements.len(), location);
            render_elements.extend(elements);
        }
        debug!("Total render elements: {}", render_elements.len());

        // Get the backend and surface data
        let backend = self.backend_data.backends.get_mut(&node).ok_or("Backend not found")?;
        let surface_data = backend.surfaces.get_mut(&crtc).ok_or("Surface not found")?;

        // Nord theme background color (nord0: #2e3440)
        let clear_color = Color32F::new(
            0x2e as f32 / 255.0,
            0x34 as f32 / 255.0,
            0x40 as f32 / 255.0,
            1.0,
        );

        // Render the frame
        match surface_data.drm_output.render_frame::<Renderer<'_>, RenderElement<'_>>(
            &mut renderer,
            &render_elements,
            clear_color,
            FrameFlags::empty(),
        ) {
            Ok(render_result) => {
                // Queue the frame (even if empty, to show background)
                if let Err(e) = surface_data.drm_output.queue_frame(()) {
                    warn!("Failed to queue frame: {:?}", e);
                }
            }
            Err(e) => {
                warn!("Failed to render frame: {:?}", e);
            }
        }

        // Send frame callbacks to windows
        let time = self.clock.now().as_millis() as u32;
        for (surface, _) in &window_surfaces {
            send_frames_surface_tree_drm(surface, time);
        }

        Ok(())
    }
}

/// Send frame callbacks to a surface tree (helper for DRM backend)
fn send_frames_surface_tree_drm(
    surface: &smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
    time: u32,
) {
    use smithay::wayland::compositor::{with_surface_tree_downward, TraversalAction};

    with_surface_tree_downward(
        surface,
        (),
        |_, _, _| TraversalAction::DoChildren(()),
        |surface, states, _| {
            use smithay::wayland::compositor::SurfaceAttributes;
            use std::cell::RefCell;

            // Send frame callback
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
        |_, _, _| true,
    );
}
