use crate::compositor::CodeVerseCompositor;
use smithay::{
    reexports::wayland_server::protocol::wl_seat::WlSeat,
    utils::Serial,
    wayland::shell::xdg::{
        PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
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

    fn new_popup(&mut self, surface: PopupSurface, positioner: PositionerState) {
        info!("New popup created");

        surface.with_pending_state(|state| {
            state.geometry = positioner.get_geometry();
            state.positioner = positioner;
        });

        self.popups.push(surface.clone());
        surface.send_configure().ok();
    }

    fn grab(&mut self, _surface: PopupSurface, _seat: WlSeat, _serial: Serial) {
        // Popup grabs are acknowledged but we don't implement exclusive
        // grab semantics yet; the popup still renders and receives input
        // through normal surface focus.
    }

    fn reposition_request(
        &mut self,
        surface: PopupSurface,
        positioner: PositionerState,
        token: u32,
    ) {
        surface.with_pending_state(|state| {
            state.geometry = positioner.get_geometry();
            state.positioner = positioner;
        });
        surface.send_repositioned(token);
        surface.send_configure().ok();
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        self.handle_toplevel_closed(&surface);
    }

    fn popup_destroyed(&mut self, surface: PopupSurface) {
        info!("Popup destroyed");
        self.popups.retain(|p| p.wl_surface() != surface.wl_surface());
    }
}
