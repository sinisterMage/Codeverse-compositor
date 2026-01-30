use crate::compositor::CodeVerseCompositor;
use smithay::wayland::selection::SelectionHandler;

impl<BackendData: 'static> SelectionHandler for CodeVerseCompositor<BackendData> {
    type SelectionUserData = ();
}
