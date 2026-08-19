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
/// `a`, `b` and `c` rather than left, centre and right: that is what is printed
/// on both boards, it is what the person pressing one reads, and it keeps the
/// row clear of [`Button::Right`], which is a direction and not a switch.
///
/// Named fields rather than five positional arguments: they are all the same
/// type, so a firmware that swaps two of them compiles cleanly and then
/// behaves wrongly on hardware, which is the worst place to discover it.
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
/// see [`Buttons::new`]. Such a key is still sampled, never reported.
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
pub struct Buttons {
    keys: [Key; 5],
}

impl Buttons {
    /// Maps the five switches onto logical buttons.
    ///
    /// By meaning, never by position — that is the framework's own contract,
    /// and it is why a screen can ask for `Confirm` without knowing there are
    /// five buttons rather than a touchscreen.
    ///
    /// **A goes back and B confirms**, which is the order the framework's own
    /// row has always been in — `Tokens::standard_hints` is `["Back", "OK",
    /// …]`, and every reader puts Back on the leftmost key of its bottom row.
    /// A person moving between a reader and one of these boards presses the
    /// same relative position for the same thing, and the hint bar above the
    /// keys says so.
    ///
    /// **C has nothing on it.** There is no useful third direction: `Right`
    /// without a `Left` is a value that can be raised and never lowered, and
    /// walking the list is what the up/down pair is for. Rather than give it a
    /// job it does badly, it is left alone — [`Buttons::with_c`] assigns it in
    /// one line when there is something worth putting there, and
    /// `Board::BADGER_2040`'s row carries the matching `RowKey::Unassigned` so
    /// the hint bar leaves its slot blank.
    pub fn new(pins: ButtonPins) -> Self {
        Buttons {
            keys: [
                Key::new(pins.a, Some(Button::Back)),
                Key::new(pins.b, Some(Button::Confirm)),
                Key::new(pins.c, None),
                Key::new(pins.up, Some(Button::Up)),
                Key::new(pins.down, Some(Button::Down)),
            ],
        }
    }

    /// Gives the third key a meaning.
    ///
    /// ```text
    /// let buttons = Buttons::new(pins).with_c(Button::Down);
    /// ```
    ///
    /// Change the board's row to match — `RowKey::Unassigned` becomes whatever
    /// this is — or the key will work while its hint slot stays blank.
    pub fn with_c(mut self, button: Button) -> Self {
        self.keys[2].button = Some(button);
        self
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
    ///
    /// `back_leads_somewhere` is the caller's answer to "is there a screen
    /// underneath this one". See the note on [`Buttons::poll`]'s use in
    /// `frame.rs`: on a device the answer decides whether Back is a key at all.
    pub fn poll<D: DrawTarget>(&mut self, backend: &Backend<D>, back_leads_somewhere: bool) {
        for key in &mut self.keys {
            // **Sampled first, and always.** A key that is skipped leaves its
            // debouncer holding a stale level, so the first press after it
            // becomes deliverable is not an edge and is swallowed — a fault
            // that would surface months later, when someone assigns C, and
            // look like a dead switch.
            let edge = key.debouncer.update(key.pin.is_high());

            let Some(button) = key.button else { continue };

            // A root screen on a board has nowhere to go back to. `Button::Back`
            // finishes the current screen, and finishing the *last* one empties
            // the stack, ends the frame loop and parks the board — which from
            // the outside is indistinguishable from a firmware that crashed,
            // because every other key stops answering too. A phone can afford
            // that key because something owns the screen underneath it; here
            // nothing does, so the press is simply not delivered.
            if button == Button::Back && !back_leads_somewhere {
                continue;
            }

            match edge {
                Some(Edge::Rising) => backend.press(button),
                Some(Edge::Falling) => backend.release(button),
                None => {}
            }
        }
    }
}
