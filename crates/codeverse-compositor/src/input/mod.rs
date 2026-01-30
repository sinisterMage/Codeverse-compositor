pub mod keyboard;
pub mod pointer;

pub use keyboard::handle_keyboard_shortcut;
pub use pointer::{handle_pointer_axis, handle_pointer_button, handle_pointer_motion};
