use crate::compositor::CodeVerseCompositor;
use smithay::{
    delegate_xdg_shell,
    reexports::wayland_server::protocol::wl_seat::WlSeat,
    utils::Serial,
    wayland::shell::xdg::{
        PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
        XdgToplevelSurfaceData,
    },
};
use tracing::info;

impl<BackendData: 'static> XdgShellHandler for CodeVerseCompositor<BackendData> {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        self.handle_new_toplevel(surface);
    }

    fn new_popup(&mut self, _surface: PopupSurface, _positioner: PositionerState) {
        info!("New popup created (not yet implemented)");
        // TODO: Handle popups in Phase 2
    }

    fn grab(&mut self, _surface: PopupSurface, _seat: WlSeat, _serial: Serial) {
        info!("Popup grab request");
        // TODO: Handle popup grabs
    }

    fn reposition_request(
        &mut self,
        _surface: PopupSurface,
        _positioner: PositionerState,
        _token: u32,
    ) {
        info!("Popup reposition request");
        // TODO: Handle popup repositioning
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        self.handle_toplevel_closed(&surface);
    }

    fn popup_destroyed(&mut self, _surface: PopupSurface) {
        info!("Popup destroyed");
        // TODO: Handle popup destruction
    }
}
