use crate::compositor::CodeVerseCompositor;
use smithay::wayland::output::OutputHandler;

impl<BackendData: 'static> OutputHandler for CodeVerseCompositor<BackendData> {}
