use crate::compositor::CodeVerseCompositor;
use smithay::delegate_viewporter;

delegate_viewporter!(@<BackendData: 'static> CodeVerseCompositor<BackendData>);
