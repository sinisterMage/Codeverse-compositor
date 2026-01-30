use crate::compositor::{ClientState, CodeVerseCompositor};
use smithay::{
    backend::{
        allocator::{
            gbm::{GbmAllocator, GbmBufferFlags, GbmDevice},
            Fourcc,
        },
        drm::{
            exporter::gbm::GbmFramebufferExporter,
            output::{DrmOutput, DrmOutputManager, DrmOutputRenderElements},
            DrmDevice, DrmDeviceFd, DrmEvent, DrmNode, NodeType,
        },
        egl::{EGLContext, EGLDevice, EGLDisplay},
        input::InputEvent,
        libinput::{LibinputInputBackend, LibinputSessionInterface},
        renderer::{
            element::{AsRenderElements, solid::SolidColorRenderElement},
            gles::{Capability, GlesRenderer},
            multigpu::{gbm::GbmGlesBackend, GpuManager, MultiRenderer},
        },
        session::{
            libseat::{self, LibSeatSession},
            Event as SessionEvent, Session,
        },
        udev::{all_gpus, primary_gpu, UdevBackend, UdevEvent},
    },
    output::{Mode as WlMode, Output, PhysicalProperties, Subpixel},
    reexports::{
        calloop::{EventLoop, LoopHandle, RegistrationToken},
        drm::control::{connector, crtc, Device, ModeTypeFlags},
        input::Libinput,
        rustix::fs::OFlags,
        wayland_server::{Display, DisplayHandle},
    },
    utils::{DeviceFd, Point, Rectangle, Transform, Physical, Size},
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

// Type alias for our render element
type OutputRenderElement = SolidColorRenderElement;

struct BackendData {
    _registration_token: RegistrationToken,
    drm_output_manager: DrmOutputManager<GbmAllocator<DrmDeviceFd>, GbmFramebufferExporter<DrmDeviceFd>, OutputRenderElement, DrmDeviceFd>,
    drm_scanner: DrmScanner,
    render_node: Option<DrmNode>,
    surfaces: HashMap<crtc::Handle, SurfaceData>,
}

struct SurfaceData {
    _output: Output,
    drm_output: DrmOutput<GbmAllocator<DrmDeviceFd>, GbmFramebufferExporter<DrmDeviceFd>, OutputRenderElement, DrmDeviceFd>,
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
        .insert_source(libinput_backend, move |event, _, _compositor| {
            // TODO: Process input events properly
            match event {
                InputEvent::Keyboard { .. } => {
                    debug!("Keyboard event");
                }
                InputEvent::PointerMotion { .. } => {
                    debug!("Pointer motion event");
                }
                InputEvent::PointerButton { .. } => {
                    debug!("Pointer button event");
                }
                _ => {}
            }
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
        event_loop.dispatch(Some(Duration::from_millis(16)), &mut compositor)?;
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
            move |event, _metadata, _compositor| match event {
                DrmEvent::VBlank(crtc) => {
                    debug!("VBlank on {:?} crtc {:?}", node_clone, crtc);
                    // TODO: Handle frame completion
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

    let drm_output = match backend
        .drm_output_manager
        .lock()
        .initialize_output::<Renderer<'_>, OutputRenderElement>(
            crtc,
            drm_mode,
            &[connector.handle()],
            &output,
            planes,
            &mut renderer,
            &DrmOutputRenderElements::default(),
        ) {
        Ok(output) => output,
        Err(err) => {
            error!("Failed to initialize drm output: {}", err);
            return;
        }
    };

    backend.surfaces.insert(
        crtc,
        SurfaceData {
            _output: output.clone(),
            drm_output,
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
}
