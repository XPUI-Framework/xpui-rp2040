//! Five buttons, as `xpui` sees them.
//!
//! Both boards here carry the same five switches and no touchscreen, wired the
//! same way: a GPIO pulled down, taken to 3V3 through the switch, so a press
//! reads high. What they do not share is which pin is which, so the pins
//! arrive named and are matched to the board's own keys here, once.

use debouncr::{DebouncerStateful, Edge, Repeat4, debounce_stateful_4};
use embassy_rp::gpio::Input;
use embedded_graphics::draw_target::DrawTarget;
use rtt_target::rprintln;
use xpui::Button;
use xpui_boards_core::Board;
use xpui_eg::Backend;

/// The five switches, one field per switch.
///
/// `a`, `b` and `c` rather than left, centre and right: that is what is printed
/// on both badges, it is what the person pressing one reads, and it keeps the
/// row clear of [`Button::Right`], which is a direction and not a switch.
///
/// Named fields rather than positional: all five are one type, and a swap
/// would compile.
pub struct ButtonPins {
    pub a: Input<'static>,
    pub b: Input<'static>,
    pub c: Input<'static>,
    pub up: Input<'static>,
    pub down: Input<'static>,
}

/// One switch, debounced, and what it means.
///
/// `button` is optional because a board can have a key with nothing on it —
/// the third switch on both badges is one. Such a key is still sampled, never
/// reported.
struct Key {
    pin: Input<'static>,
    debouncer: DebouncerStateful<u8, Repeat4>,
    button: Option<Button>,
}

impl Key {
    fn new(pin: Input<'static>, button: Option<Button>) -> Self {
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
pub(crate) struct Buttons {
    keys: [Key; 5],
}

impl Buttons {
    /// Asks the board what the key beside each pin sends.
    ///
    /// **The firmware does not decide what a key means.** The names below are
    /// matched exactly against the board's own key labels — `down` is the
    /// field, `Dn` is the board's name — and a name the board does not carry
    /// resolves to `None`, so that switch is sampled and silent. Each says
    /// what it resolved to at boot, so `key up sends None` is what a typo
    /// looks like; a pin behind the wrong name is what only a thumb can find.
    pub(crate) fn new(board: Board, pins: ButtonPins) -> Self {
        let sends = |label: &str| {
            board
                .bezel
                .and_then(|bezel| bezel.button_labelled(label))
                .and_then(|key| key.action.button())
        };

        let keys = [
            ("a", pins.a),
            ("b", pins.b),
            ("c", pins.c),
            ("Up", pins.up),
            ("Dn", pins.down),
        ]
        .map(|(label, pin)| {
            let button = sends(label);
            rprintln!("xpui: key {} sends {:?}", label, button);
            Key::new(pin, button)
        });

        Buttons { keys }
    }

    /// Samples every switch once, and reports only what changed.
    ///
    /// Edges, not levels: a button reported as pressed on every frame re-fires
    /// whatever it is on, so a finger resting on Down would walk a list to the
    /// bottom in a fraction of a second. Four agreeing samples make a state
    /// change, about 40 ms at the frame interval. Every switch is reported,
    /// Back included: whether finishing the root screen is allowed is
    /// [`App::keep_root`](xpui::App::keep_root)'s answer, not a pin's.
    pub(crate) fn poll<D: DrawTarget>(&mut self, backend: &Backend<D>) {
        for key in &mut self.keys {
            // **Sampled first, and always.** A key that is skipped leaves its
            // debouncer holding a stale level, so the first press after it
            // becomes deliverable is not an edge and is swallowed — a fault
            // that would surface months later, when someone assigns C, and
            // look like a dead switch.
            let edge = key.debouncer.update(key.pin.is_high());

            let Some(button) = key.button else { continue };

            match edge {
                Some(Edge::Rising) => backend.press(button),
                Some(Edge::Falling) => backend.release(button),
                None => {}
            }
        }
    }
}
