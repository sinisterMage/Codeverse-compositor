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

            // Find the window that owns this surface
            if let Some(window_id) = self
                .window_tree
                .find_windows()
                .into_iter()
                .find(|&window_id| {
                    if let Some(container) = self.window_tree.get(window_id) {
                        if let Some(ref window_handle) = container.window {
                            return window_handle.wl_surface() == &root;
                        }
                    }
                    false
                })
            {
                debug!("Surface committed for window {:?}", window_id);

                // Only trigger layout recalculation if we have cached screen geometry
                // The actual rendering loop will set the screen geometry and do initial layout
                if let Some(screen_rect) = self.last_screen_geometry {
                    // Check if the window's geometry is uninitialized (0x0)
                    // to avoid unnecessary re-layout on every commit
                    let needs_layout = self.window_tree.get(window_id)
                        .map(|c| c.geometry.width == 0 || c.geometry.height == 0)
                        .unwrap_or(false);

                    if needs_layout {
                        if let Some(ref mut manager) = self.workspace_manager {
                            let gap_width = self.config.general.gap_width as i32;
                            manager.layout_active_workspace(&mut self.window_tree, screen_rect, gap_width);
                        }
                    }
                }
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
