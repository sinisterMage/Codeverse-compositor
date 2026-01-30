use crate::compositor::CodeVerseCompositor;
use smithay::{
    delegate_data_device,
    wayland::selection::data_device::{
        ClientDndGrabHandler, DataDeviceHandler, DataDeviceState, ServerDndGrabHandler,
    },
};

impl<BackendData: 'static> DataDeviceHandler for CodeVerseCompositor<BackendData> {
    fn data_device_state(&mut self) -> &mut DataDeviceState {
        &mut self.data_device_state
    }
}

impl<BackendData: 'static> ClientDndGrabHandler for CodeVerseCompositor<BackendData> {}

impl<BackendData: 'static> ServerDndGrabHandler for CodeVerseCompositor<BackendData> {}
