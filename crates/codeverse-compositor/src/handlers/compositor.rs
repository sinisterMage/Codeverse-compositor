use crate::compositor::CodeVerseCompositor;
use smithay::{
    backend::renderer::utils::on_commit_buffer_handler,
    delegate_compositor,
    wayland::{
        buffer::BufferHandler,
        compositor::{get_parent, is_sync_subsurface, CompositorHandler, CompositorState},
    },
};
use tracing::debug;

impl<BackendData: 'static> CompositorHandler for CodeVerseCompositor<BackendData> {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(
        &self,
        client: &'a smithay::reexports::wayland_server::Client,
    ) -> &'a smithay::wayland::compositor::CompositorClientState {
        &client.get_data::<crate::compositor::ClientState>().unwrap().compositor_state
    }

    fn commit(&mut self, surface: &smithay::reexports::wayland_server::protocol::wl_surface::WlSurface) {
        on_commit_buffer_handler::<Self>(surface);

        // Handle subsurface synchronization
        if !is_sync_subsurface(surface) {
            let mut root = surface.clone();
            while let Some(parent) = get_parent(&root) {
                root = parent;
            }
            if let Some(window) = self
                .window_tree
                .find_windows()
                .into_iter()
                .find(|_| {
                    // TODO: Phase 2 - match window surface
                    false
                })
            {
                // TODO: Phase 2 - handle window updates
                debug!("Window committed: {:?}", window);
            }
        }
    }
}

impl<BackendData: 'static> BufferHandler for CodeVerseCompositor<BackendData> {
    fn buffer_destroyed(
        &mut self,
        _buffer: &smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer,
    ) {
        // Buffer destroyed, nothing to do for now
    }
}
