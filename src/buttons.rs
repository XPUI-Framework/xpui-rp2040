//! Five buttons, as `xpui` sees them.
//!
//! Both boards here carry the same five switches and no touchscreen, wired the
//! same way: a GPIO pulled down, taken to 3V3 through the switch, so a press
//! reads high. What they do not share is which pin is which, so the pins
//! arrive named and the mapping to a logical button happens here, once.

use debouncr::{DebouncerStateful, Edge, Repeat4, debounce_stateful_4};
use embassy_rp::gpio::Input;
use embedded_graphics::draw_target::DrawTarget;
use xpui::Button;
use xpui_eg::Backend;

/// The five switches, by the label silk-screened beside them.
///
/// Named fields rather than five positional arguments: they are all the same
/// type, so a firmware that swaps two of them compiles cleanly and then
/// behaves wrongly on hardware, which is the worst place to discover it.
pub struct ButtonPins {
    pub left: Input<'static>,
    pub centre: Input<'static>,
    pub right: Input<'static>,
    pub up: Input<'static>,
    pub down: Input<'static>,
}

/// One switch, debounced, and what it means.
struct Key {
    pin: Input<'static>,
    debouncer: DebouncerStateful<u8, Repeat4>,
    button: Button,
}

impl Key {
    fn new(pin: Input<'static>, button: Button) -> Self {
        Key {
            pin,
            // Nothing is held at boot. Claiming otherwise would make the first
            // real press read as a release.
            debouncer: debounce_stateful_4(false),
            button,
        }
    }
}

/// The board's switches, ready to be sampled once a frame.
pub struct Buttons {
    keys: [Key; 5],
}

impl Buttons {
    /// Maps the five switches onto logical buttons.
    ///
    /// By meaning, never by position — that is the framework's own contract, and
    /// it is why a screen can ask for `Confirm` without knowing there are five
    /// buttons rather than a touchscreen. Centre confirms and left goes back,
    /// which is where a thumb expects them on a badge held in one hand. Right
    /// stays `Right` rather than becoming a second confirm, so a stepper and a
    /// slider have something to increment with.
    pub fn new(pins: ButtonPins) -> Self {
        Buttons {
            keys: [
                Key::new(pins.left, Button::Back),
                Key::new(pins.centre, Button::Confirm),
                Key::new(pins.right, Button::Right),
                Key::new(pins.up, Button::Up),
                Key::new(pins.down, Button::Down),
            ],
        }
    }

    /// Samples every switch once, and reports only what changed.
    ///
    /// Edges, not levels. `xpui`'s input is edge-based: a button reported as
    /// pressed on every frame re-fires whatever it is on, so a finger resting
    /// on Down would walk a list to the bottom in a fraction of a second.
    ///
    /// Four agreeing samples make a state change, so at the frame interval the
    /// loop runs at a press settles in about 40 ms — long enough to swallow
    /// contact bounce, short enough not to be felt.
    pub fn poll<D: DrawTarget>(&mut self, backend: &Backend<D>) {
        for key in &mut self.keys {
            match key.debouncer.update(key.pin.is_high()) {
                Some(Edge::Rising) => backend.press(key.button),
                Some(Edge::Falling) => backend.release(key.button),
                None => {}
            }
        }
    }
}
