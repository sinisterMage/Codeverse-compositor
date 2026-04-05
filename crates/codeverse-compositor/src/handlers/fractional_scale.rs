use crate::compositor::CodeVerseCompositor;
use smithay::{
    delegate_fractional_scale,
    wayland::fractional_scale::FractionalScaleHandler,
};

impl<BackendData: 'static> FractionalScaleHandler for CodeVerseCompositor<BackendData> {
    fn new_fractional_scale(
        &mut self,
        _surface: smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
    ) {
        // Scale is applied per-output; for now we use 1.0 (integer) as default.
        // When an output scale is configured via config.outputs, this callback
        // can be extended to set the preferred fractional scale.
    }
}

delegate_fractional_scale!(@<BackendData: 'static> CodeVerseCompositor<BackendData>);
