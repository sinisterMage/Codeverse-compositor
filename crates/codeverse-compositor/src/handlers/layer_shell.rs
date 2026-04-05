use crate::compositor::CodeVerseCompositor;
use smithay::{
    delegate_layer_shell,
    reexports::wayland_server::protocol::wl_output::WlOutput,
    utils::Size,
    wayland::shell::wlr_layer::{
        Layer, LayerSurface, WlrLayerShellHandler, WlrLayerShellState,
    },
};
use tracing::info;

impl<BackendData: 'static> WlrLayerShellHandler for CodeVerseCompositor<BackendData> {
    fn shell_state(&mut self) -> &mut WlrLayerShellState {
        &mut self.layer_shell_state
    }

    fn new_layer_surface(
        &mut self,
        surface: LayerSurface,
        _output: Option<WlOutput>,
        _layer: Layer,
        _namespace: String,
    ) {
        info!("New layer surface created");

        surface.with_pending_state(|state| {
            let size = state.size.unwrap_or_default();
            if size.w == 0 || size.h == 0 {
                if let Some(screen) = self.last_screen_geometry {
                    let w = if size.w == 0 { screen.width as i32 } else { size.w };
                    let h = if size.h == 0 { 32 } else { size.h };
                    state.size = Some(Size::from((w, h)));
                }
            }
        });
        surface.send_configure();

        self.layer_surfaces.push(surface);
    }

    fn layer_destroyed(&mut self, surface: LayerSurface) {
        info!("Layer surface destroyed");
        self.layer_surfaces.retain(|s| s.wl_surface() != surface.wl_surface());
    }
}

delegate_layer_shell!(@<BackendData: 'static> CodeVerseCompositor<BackendData>);
