# Your first screen on a board

You have a Badger 2040 — 296×128 of monochrome e-ink, five buttons, 2 MB of
flash, no touchscreen. This puts a screen on it.

It assumes you have written one for the simulator already;
[the framework's tutorial](../../../crates/xpui/docs/tutorial.md) is that, and
nothing here repeats it. **What is different on a board is the subject.**

Every Rust block below is compiled by `cargo test`. The device-only ones — an
embassy entry point, a flash command — are fenced `text`, and each says why it
cannot be compiled here.

## Nothing about the screen changes

That is the whole claim, so it goes first. This is a complete screen, and the
same source runs in the simulator window, on a Badger, on a Tufty and inside a
C++ firmware:

```rust
use xpui::screen::Screen;
use xpui::{List, ListRow, NavigationScreen, View};

pub struct Battery {
    percent: i32,
}

impl Battery {
    pub fn new() -> Self {
        Battery { percent: 72 }
    }
}

impl Screen for Battery {
    type Message = ();

    fn body(&self) -> impl View<()> {
        NavigationScreen::new(
            List::new().push(ListRow::new("Charge").value(if self.percent > 20 {
                "Good"
            } else {
                "Low"
            })),
        )
        .title("Battery")
    }

    fn update(&mut self, _message: ()) {}
}
```

No pixel offsets, no panel size, no board. If yours has any of those in it, the
rest of this will not help — take them out first.

## 1. The board is data

A `Board` is what the panel *is*: its size, the chrome that fits it, whether it
has a touchscreen, how long a refresh takes. Both the simulator and the
firmware read the same value, which is what makes "develop in a window, then
flash it" true rather than aspirational.

```rust
use xpui_boards::Board;

let badger = Board::BADGER_2040;

assert_eq!((badger.width, badger.height), (296, 128));
assert!(!badger.touch);
// A full refresh, in milliseconds. The frame loop uses it to decide how often
// painting is worth attempting at all.
assert_eq!(badger.refresh_ms, 900);
```

Build the backend from it and the chrome sizes itself:

```rust
use xpui_boards::Board;
use xpui_eg::{Backend, Framebuffer, Palette};

let badger = Board::BADGER_2040;
let backend = Backend::for_board(
    Framebuffer::new(badger.width, badger.height),
    badger,
    // This board's polarity, and it is not the obvious one — step 2.
    Palette::INK_IS_OFF,
);

// The board's tokens carry its UI scale, and the faces are chosen to fit them
// — so a 296x128 strip does not get type taller than its own hint band.
assert_eq!(backend.board(), Some(badger));
```

> `Framebuffer` is the host-side target these snippets draw into so they can be
> tested. On the board it is the panel driver — see step 3.

## 2. The trap that inverts your panel

**`Palette` decides which colour is ink, and getting it wrong is invisible
until the board is in your hand.**

```rust
use xpui_eg::Palette;

// Most targets: ink is "on".
let usual = Palette::INK_IS_ON;
// The Badger's uc8151 maps BinaryColor::Off to black, so ink is "off".
let badger = Palette::INK_IS_OFF;

assert_ne!(usual.ink, badger.ink);
```

Both compile. Both run. The wrong one gives you a fully inverted panel — white
text on black, every screen — with no error anywhere, because nothing in the
framework knows which way your driver wired it.

The Badger needs `INK_IS_OFF`, which is why step 1 used it. If your first flash
comes up inverted, this is why, and it is a one-word fix.

Neither constant fits a colour panel: they are `Palette<BinaryColor>`, and the
Tufty's ST7789 is `Rgb565`. Build one from the two colours you want instead —
`Palette::new(Rgb565::BLACK, Rgb565::WHITE)`, as
[`src/bin/tufty2040.rs`](../src/bin/tufty2040.rs) does. The framework paints in
ink and background and has no way to ask for a third colour, so running it on
colour hardware is a matter of choosing which two.

## 3. The frame loop

On a device you own the loop. It is short, and every line of it is there for a
reason that costs you if you drop it:

```text
loop {
    backend.begin_frame(now_millis());   // advance the clock, clear input edges
    buttons.poll(backend);               // your GPIO -> logical buttons

    app.tick();                          // one frame of input
    if app.render_if_dirty() {           // paint ONLY if something changed
        backend.clear_dirty();
        backend.with_display(|panel| panel.update());   // ~800 ms on e-ink
    }

    Timer::after(FRAME_INTERVAL).await;  // 10 ms
}
```

Fenced `text` because it is the body of an `async fn` that needs embassy and a
panel — [`src/frame.rs`](../src/frame.rs) is the compiled version, and it is
this with the types filled in.

Three things about it:

**Paint only when the framework asks.** A full refresh on the Badger is close
to a second, during which the panel is mid-update and the buttons are unread. A
loop that presented every frame would be permanently unusable.

**The wait is not optional.** A loop with nothing in it spins the core at full
clock for the life of the battery. Ten milliseconds is also what sets the
debounce window: four agreeing samples is 40 ms.

**A suspending driver needs `loan_display`, not `with_display`.** The closure
above holds the backend's state for as long as it runs, so a flush you have to
`await` cannot go inside one. `frame.rs`'s `run_async` is the shape for that;
the Badger does not need it, because the published `uc8151` blocks.

## 4. What you have to fit in

2 MB of flash and 264 kB of SRAM, and the framework does not hide either.

| | |
|---|---|
| Heap | 64 kB, in [`src/runtime.rs`](../src/runtime.rs) — the leaked backend, the screen stack, and the view tree `body()` rebuilds every frame |
| The Badger's framebuffer | 4,736 bytes, inside the backend |
| The whole firmware | around 220 kB of flash |

The rule that keeps you inside it: **`body()` runs on every paint and on every
frame carrying input.** Build `String`s when the screen is built, not while
describing it — a `format!` in `body()` allocates several times a second and
drags `core::fmt` onto a path that has to stay cheap.

```rust
use alloc::string::String;
extern crate alloc;

pub struct Reading {
    // Formatted once, when the value changes.
    label: String,
}

impl Reading {
    pub fn set(&mut self, percent: i32) {
        self.label = alloc::format!("{percent}%");
    }

    // `body()` borrows it. No allocation, no formatting.
    pub fn label(&self) -> &str {
        &self.label
    }
}
```

## 5. Flash it

```text
# With a debug probe — a Raspberry Pi Debug Probe, or a Pico running picoprobe.
cargo run --release --bin badger2040

# Without one: hold BOOTSEL while plugging the board in, then
cargo build --release --bin badger2040
elf2uf2-rs -d target/thumbv6m-none-eabi/release/badger2040
```

Fenced `text`: these move a binary onto hardware, which is not something a test
can do.

Run both from [`examples/rp2040/`](../), whose `.cargo/config.toml` sets the
target and the runner. This crate is its own workspace, so from the repository
root `cargo build` does not reach it at all — use `--manifest-path`.

## What has been proven, and where

Both boards here have been run over a debug probe. The firmware says what it
found on the way up — board size against panel size, what each key resolved to,
and the heap after the first frame — because a driver a quarter turn out lays
out plausibly and puts the screen in a corner of the glass, and a key looked up
under a name the board does not carry is otherwise simply silent.

Three faults came out of that first run, and each is worth knowing before you
meet it on your own board:

- **A slow panel makes auto-repeat lie.** The loop is blind for the ~800 ms a
  refresh takes, and the button still reads as down on the frame after. The
  framework no longer credits a gap it could not see through — but if you write
  your own loop, that is the trap.
- **`Button::Back` on a root screen would end the app**, which on a device
  means the loop exits and the board parks. `App::keep_root()` is the opt-in
  that declines it — see [`src/frame.rs`](../src/frame.rs). Decline the *pop*,
  never the key: a screen may claim Back for itself and an open value cancels
  with it, and a loop that drops the key at the pin takes both away.
- **`mipidsi` shortens a run of one colour into a bare strobe loop** that
  outruns an ST7789 over a parallel bus, and only black and white take that
  path. `PacedFill` in `xpui-embedded-graphics` is the wrapper that avoids it, and
  the whole story is in its module docs.

What has *not* been tried is battery operation: both boards have only been run
over USB, where the Badger's GP10 holds up a rail the USB supply is feeding
anyway. See [the README's pin tables](../README.md#pins) for what every pin on
both boards does.
