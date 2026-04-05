use crate::compositor::CodeVerseCompositor;
use smithay::{
    backend::allocator::dmabuf::Dmabuf,
    delegate_dmabuf,
    wayland::dmabuf::{DmabufGlobal, DmabufHandler, DmabufState, ImportNotifier},
};

impl<BackendData: 'static> DmabufHandler for CodeVerseCompositor<BackendData> {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        &mut self.dmabuf_state
    }

    fn dmabuf_imported(
        &mut self,
        _global: &DmabufGlobal,
        _dmabuf: Dmabuf,
        notifier: ImportNotifier,
    ) {
        // For the winit backend, the renderer is not easily accessible here,
        // so we optimistically mark the import as successful and let the
        // renderer fail gracefully at draw time if the buffer is invalid.
        // The DRM backend will also go through this path; Smithay's
        // MultiRenderer handles format negotiation at the protocol level.
        let _ = notifier.successful::<CodeVerseCompositor<BackendData>>();
    }
}

delegate_dmabuf!(@<BackendData: 'static> CodeVerseCompositor<BackendData>);
