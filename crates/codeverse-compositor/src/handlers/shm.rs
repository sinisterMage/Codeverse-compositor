use crate::compositor::CodeVerseCompositor;
use smithay::wayland::shm::{ShmHandler, ShmState};

impl<BackendData: 'static> ShmHandler for CodeVerseCompositor<BackendData> {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}
