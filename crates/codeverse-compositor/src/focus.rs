use smithay::{
    input::{
        keyboard::{KeyboardTarget, KeysymHandle, ModifiersState},
        pointer::{
            AxisFrame, ButtonEvent, GestureHoldBeginEvent, GestureHoldEndEvent,
            GesturePinchBeginEvent, GesturePinchEndEvent, GesturePinchUpdateEvent,
            GestureSwipeBeginEvent, GestureSwipeEndEvent, GestureSwipeUpdateEvent,
            MotionEvent, PointerTarget, RelativeMotionEvent,
        },
        Seat, SeatHandler,
    },
    reexports::wayland_server::{protocol::wl_surface::WlSurface, Resource},
    utils::{IsAlive, Serial},
    wayland::seat::WaylandFocus,
};
use std::borrow::Cow;

use crate::compositor::CodeVerseCompositor;

/// Focus target for keyboard input
#[derive(Debug, Clone, PartialEq)]
pub enum KeyboardFocusTarget {
    /// A regular Wayland surface
    Surface(WlSurface),
}

impl IsAlive for KeyboardFocusTarget {
    fn alive(&self) -> bool {
        match self {
            KeyboardFocusTarget::Surface(s) => s.alive(),
        }
    }
}

impl<BackendData: 'static> KeyboardTarget<CodeVerseCompositor<BackendData>> for KeyboardFocusTarget {
    fn enter(
        &self,
        seat: &Seat<CodeVerseCompositor<BackendData>>,
        data: &mut CodeVerseCompositor<BackendData>,
        keys: Vec<KeysymHandle<'_>>,
        serial: Serial,
    ) {
        match self {
            KeyboardFocusTarget::Surface(s) => {
                KeyboardTarget::enter(s, seat, data, keys, serial);
            }
        }
    }

    fn leave(
        &self,
        seat: &Seat<CodeVerseCompositor<BackendData>>,
        data: &mut CodeVerseCompositor<BackendData>,
        serial: Serial,
    ) {
        match self {
            KeyboardFocusTarget::Surface(s) => {
                KeyboardTarget::leave(s, seat, data, serial);
            }
        }
    }

    fn key(
        &self,
        seat: &Seat<CodeVerseCompositor<BackendData>>,
        data: &mut CodeVerseCompositor<BackendData>,
        key: KeysymHandle<'_>,
        state: smithay::backend::input::KeyState,
        serial: Serial,
        time: u32,
    ) {
        match self {
            KeyboardFocusTarget::Surface(s) => {
                KeyboardTarget::key(s, seat, data, key, state, serial, time);
            }
        }
    }

    fn modifiers(
        &self,
        seat: &Seat<CodeVerseCompositor<BackendData>>,
        data: &mut CodeVerseCompositor<BackendData>,
        modifiers: ModifiersState,
        serial: Serial,
    ) {
        match self {
            KeyboardFocusTarget::Surface(s) => {
                KeyboardTarget::modifiers(s, seat, data, modifiers, serial);
            }
        }
    }
}

impl WaylandFocus for KeyboardFocusTarget {
    fn wl_surface(&self) -> Option<Cow<'_, WlSurface>> {
        match self {
            KeyboardFocusTarget::Surface(s) => Some(Cow::Owned(s.clone())),
        }
    }
}

/// Focus target for pointer input
#[derive(Debug, Clone, PartialEq)]
pub enum PointerFocusTarget {
    /// A regular Wayland surface
    Surface(WlSurface),
}

impl IsAlive for PointerFocusTarget {
    fn alive(&self) -> bool {
        match self {
            PointerFocusTarget::Surface(s) => s.alive(),
        }
    }
}

impl<BackendData: 'static> PointerTarget<CodeVerseCompositor<BackendData>> for PointerFocusTarget {
    fn enter(
        &self,
        seat: &Seat<CodeVerseCompositor<BackendData>>,
        data: &mut CodeVerseCompositor<BackendData>,
        event: &MotionEvent,
    ) {
        match self {
            PointerFocusTarget::Surface(s) => PointerTarget::enter(s, seat, data, event),
        }
    }

    fn motion(
        &self,
        seat: &Seat<CodeVerseCompositor<BackendData>>,
        data: &mut CodeVerseCompositor<BackendData>,
        event: &MotionEvent,
    ) {
        match self {
            PointerFocusTarget::Surface(s) => PointerTarget::motion(s, seat, data, event),
        }
    }

    fn relative_motion(
        &self,
        seat: &Seat<CodeVerseCompositor<BackendData>>,
        data: &mut CodeVerseCompositor<BackendData>,
        event: &RelativeMotionEvent,
    ) {
        match self {
            PointerFocusTarget::Surface(s) => PointerTarget::relative_motion(s, seat, data, event),
        }
    }

    fn button(
        &self,
        seat: &Seat<CodeVerseCompositor<BackendData>>,
        data: &mut CodeVerseCompositor<BackendData>,
        event: &ButtonEvent,
    ) {
        match self {
            PointerFocusTarget::Surface(s) => PointerTarget::button(s, seat, data, event),
        }
    }

    fn axis(
        &self,
        seat: &Seat<CodeVerseCompositor<BackendData>>,
        data: &mut CodeVerseCompositor<BackendData>,
        frame: AxisFrame,
    ) {
        match self {
            PointerFocusTarget::Surface(s) => PointerTarget::axis(s, seat, data, frame),
        }
    }

    fn frame(
        &self,
        seat: &Seat<CodeVerseCompositor<BackendData>>,
        data: &mut CodeVerseCompositor<BackendData>,
    ) {
        match self {
            PointerFocusTarget::Surface(s) => PointerTarget::frame(s, seat, data),
        }
    }

    fn leave(
        &self,
        seat: &Seat<CodeVerseCompositor<BackendData>>,
        data: &mut CodeVerseCompositor<BackendData>,
        serial: Serial,
        time: u32,
    ) {
        match self {
            PointerFocusTarget::Surface(s) => PointerTarget::leave(s, seat, data, serial, time),
        }
    }

    fn gesture_swipe_begin(
        &self,
        seat: &Seat<CodeVerseCompositor<BackendData>>,
        data: &mut CodeVerseCompositor<BackendData>,
        event: &GestureSwipeBeginEvent,
    ) {
        match self {
            PointerFocusTarget::Surface(s) => PointerTarget::gesture_swipe_begin(s, seat, data, event),
        }
    }

    fn gesture_swipe_update(
        &self,
        seat: &Seat<CodeVerseCompositor<BackendData>>,
        data: &mut CodeVerseCompositor<BackendData>,
        event: &GestureSwipeUpdateEvent,
    ) {
        match self {
            PointerFocusTarget::Surface(s) => PointerTarget::gesture_swipe_update(s, seat, data, event),
        }
    }

    fn gesture_swipe_end(
        &self,
        seat: &Seat<CodeVerseCompositor<BackendData>>,
        data: &mut CodeVerseCompositor<BackendData>,
        event: &GestureSwipeEndEvent,
    ) {
        match self {
            PointerFocusTarget::Surface(s) => PointerTarget::gesture_swipe_end(s, seat, data, event),
        }
    }

    fn gesture_pinch_begin(
        &self,
        seat: &Seat<CodeVerseCompositor<BackendData>>,
        data: &mut CodeVerseCompositor<BackendData>,
        event: &GesturePinchBeginEvent,
    ) {
        match self {
            PointerFocusTarget::Surface(s) => PointerTarget::gesture_pinch_begin(s, seat, data, event),
        }
    }

    fn gesture_pinch_update(
        &self,
        seat: &Seat<CodeVerseCompositor<BackendData>>,
        data: &mut CodeVerseCompositor<BackendData>,
        event: &GesturePinchUpdateEvent,
    ) {
        match self {
            PointerFocusTarget::Surface(s) => PointerTarget::gesture_pinch_update(s, seat, data, event),
        }
    }

    fn gesture_pinch_end(
        &self,
        seat: &Seat<CodeVerseCompositor<BackendData>>,
        data: &mut CodeVerseCompositor<BackendData>,
        event: &GesturePinchEndEvent,
    ) {
        match self {
            PointerFocusTarget::Surface(s) => PointerTarget::gesture_pinch_end(s, seat, data, event),
        }
    }

    fn gesture_hold_begin(
        &self,
        seat: &Seat<CodeVerseCompositor<BackendData>>,
        data: &mut CodeVerseCompositor<BackendData>,
        event: &GestureHoldBeginEvent,
    ) {
        match self {
            PointerFocusTarget::Surface(s) => PointerTarget::gesture_hold_begin(s, seat, data, event),
        }
    }

    fn gesture_hold_end(
        &self,
        seat: &Seat<CodeVerseCompositor<BackendData>>,
        data: &mut CodeVerseCompositor<BackendData>,
        event: &GestureHoldEndEvent,
    ) {
        match self {
            PointerFocusTarget::Surface(s) => PointerTarget::gesture_hold_end(s, seat, data, event),
        }
    }
}

impl WaylandFocus for PointerFocusTarget {
    fn wl_surface(&self) -> Option<Cow<'_, WlSurface>> {
        match self {
            PointerFocusTarget::Surface(s) => Some(Cow::Owned(s.clone())),
        }
    }
}

// TouchTarget implementation for PointerFocusTarget
use smithay::input::touch::{
    DownEvent as TouchDownEvent, MotionEvent as TouchMotionEvent, OrientationEvent, ShapeEvent,
    TouchTarget, UpEvent as TouchUpEvent,
};

impl<BackendData: 'static> TouchTarget<CodeVerseCompositor<BackendData>> for PointerFocusTarget {
    fn down(
        &self,
        seat: &Seat<CodeVerseCompositor<BackendData>>,
        data: &mut CodeVerseCompositor<BackendData>,
        event: &TouchDownEvent,
        seq: Serial,
    ) {
        match self {
            PointerFocusTarget::Surface(s) => TouchTarget::down(s, seat, data, event, seq),
        }
    }

    fn up(
        &self,
        seat: &Seat<CodeVerseCompositor<BackendData>>,
        data: &mut CodeVerseCompositor<BackendData>,
        event: &TouchUpEvent,
        seq: Serial,
    ) {
        match self {
            PointerFocusTarget::Surface(s) => TouchTarget::up(s, seat, data, event, seq),
        }
    }

    fn motion(
        &self,
        seat: &Seat<CodeVerseCompositor<BackendData>>,
        data: &mut CodeVerseCompositor<BackendData>,
        event: &TouchMotionEvent,
        seq: Serial,
    ) {
        match self {
            PointerFocusTarget::Surface(s) => TouchTarget::motion(s, seat, data, event, seq),
        }
    }

    fn frame(
        &self,
        seat: &Seat<CodeVerseCompositor<BackendData>>,
        data: &mut CodeVerseCompositor<BackendData>,
        seq: Serial,
    ) {
        match self {
            PointerFocusTarget::Surface(s) => TouchTarget::frame(s, seat, data, seq),
        }
    }

    fn cancel(
        &self,
        seat: &Seat<CodeVerseCompositor<BackendData>>,
        data: &mut CodeVerseCompositor<BackendData>,
        seq: Serial,
    ) {
        match self {
            PointerFocusTarget::Surface(s) => TouchTarget::cancel(s, seat, data, seq),
        }
    }

    fn shape(
        &self,
        seat: &Seat<CodeVerseCompositor<BackendData>>,
        data: &mut CodeVerseCompositor<BackendData>,
        event: &ShapeEvent,
        seq: Serial,
    ) {
        match self {
            PointerFocusTarget::Surface(s) => TouchTarget::shape(s, seat, data, event, seq),
        }
    }

    fn orientation(
        &self,
        seat: &Seat<CodeVerseCompositor<BackendData>>,
        data: &mut CodeVerseCompositor<BackendData>,
        event: &OrientationEvent,
        seq: Serial,
    ) {
        match self {
            PointerFocusTarget::Surface(s) => TouchTarget::orientation(s, seat, data, event, seq),
        }
    }
}

// Implement SeatHandler for CodeVerseCompositor
impl<BackendData: 'static> SeatHandler for CodeVerseCompositor<BackendData> {
    type KeyboardFocus = KeyboardFocusTarget;
    type PointerFocus = PointerFocusTarget;
    type TouchFocus = PointerFocusTarget;

    fn seat_state(&mut self) -> &mut smithay::input::SeatState<Self> {
        &mut self.seat_state
    }

    fn focus_changed(
        &mut self,
        _seat: &Seat<Self>,
        _focused: Option<&Self::KeyboardFocus>,
    ) {
        // Focus changes are tracked via window_tree.set_focused()
    }

    fn cursor_image(
        &mut self,
        _seat: &Seat<Self>,
        _image: smithay::input::pointer::CursorImageStatus,
    ) {
        // TODO: Update cursor rendering
    }
}
