use crate::compositor::CodeVerseCompositor;
use smithay::{
    delegate_xdg_decoration,
    wayland::shell::xdg::decoration::{XdgDecorationHandler, XdgDecorationState},
};
use smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode;
use tracing::info;

impl<BackendData: 'static> XdgDecorationHandler for CodeVerseCompositor<BackendData> {
    fn new_decoration(&mut self, toplevel: smithay::wayland::shell::xdg::ToplevelSurface) {
        // We're a tiling compositor — prefer client-side decorations so the
        // client draws its own title bar and we control the border.
        toplevel.with_pending_state(|state| {
            state.decoration_mode = Some(Mode::ClientSide);
        });
        toplevel.send_configure();
        info!("XDG decoration: requested client-side for new toplevel");
    }

    fn request_mode(
        &mut self,
        toplevel: smithay::wayland::shell::xdg::ToplevelSurface,
        _mode: Mode,
    ) {
        toplevel.with_pending_state(|state| {
            state.decoration_mode = Some(Mode::ClientSide);
        });
        toplevel.send_configure();
    }

    fn unset_mode(&mut self, toplevel: smithay::wayland::shell::xdg::ToplevelSurface) {
        toplevel.with_pending_state(|state| {
            state.decoration_mode = Some(Mode::ClientSide);
        });
        toplevel.send_configure();
    }
}

delegate_xdg_decoration!(@<BackendData: 'static> CodeVerseCompositor<BackendData>);
