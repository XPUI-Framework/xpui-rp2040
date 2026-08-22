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

A `Board` is what the panel and the case *are*: size, orientation, whether it
has a touchscreen, how long a refresh takes, what the keys along the bottom
mean, and how big the glass is in millimetres. Both the simulator and the
firmware read the same value, which is what makes "develop in a window, then
flash it" true rather than aspirational.

**It is not the chrome.** Nothing below your firmware knows what a board is —
not the backend, not the components that paint. Joining the two is your job,
and it is one derivation and five builder calls.

```rust
use xpui_boards::Board;

let badger = Board::BADGER_2040;

assert_eq!((badger.width, badger.height), (296, 128));
assert!(!badger.touch);
// A full refresh, in milliseconds. The frame loop uses it to decide how often
// painting is worth attempting at all.
assert_eq!(badger.refresh_ms, 900);
```

Here is the wiring. Each line answers one question, four of them with a value
the board holds and the last with the measurements the first line derived:

```rust
use xpui_boards::Board;
use xpui::host::Canvas;
use xpui_eg::{Backend, Fonts, Labels, Metrics, Palette};
use xpui_screenshot::Framebuffer;

let badger = Board::BADGER_2040;

// How big everything is. Derived from the panel's size and the board's UI
// scale; the last argument is whether to reserve the band along the bottom
// that names the keys. `!badger.touch` is `true` here — a board driven by a
// finger has no keys to name, and the band would be a strip of words for
// hardware nobody has. `gallery::wire` derives it the same way.
let metrics =
    Metrics::for_device(badger.width, badger.height, badger.ui_scale_percent, !badger.touch);

let backend = Backend::new(
    Framebuffer::new(badger.width, badger.height),
    // This board's polarity, and it is not the obvious one — step 2.
    Palette::INK_IS_OFF,
)
.with_metrics(metrics)
// What the hint band says. A 296x128 strip gets "OK" where a reader gets
// "Select", because the long word does not fit across a third of it.
.with_labels(Labels::for_panel(badger.width, badger.height))
// What the keys along the bottom mean, in order. The Badger's row is three
// slots — Back, Confirm, and one with nothing on it — because the pair that
// walks a list sits down the edge, not along the bottom. A hardware fact, so
// it comes off the board rather than out of a preset.
.with_keys(badger.keys)
// Whether Left and Right exist as keys, which decides how a value can be
// edited. The Badger has neither.
.with_left_right_keys(badger.has_left_right_keys())
// Faces chosen to fit the measurements — so a 296x128 strip does not get type
// taller than its own hint band.
.with_fonts(Fonts::for_metrics(&metrics));

// The size is the display's, whatever the board says — `Backend::new` measures
// what it is handed.
assert_eq!(backend.screen_size().width, 296);
```

The gallery packages exactly that as `gallery::wire`, and both RP2040 binaries
call it rather than repeating the block. Yours can too; write it out once if
you would rather see it.

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
  outruns an ST7789 over a parallel bus, and any pixel whose two bytes match
  takes that path — 256 of them, ink and background among them. `PacedFill` in
  `xpui-embedded-graphics` is the wrapper that avoids it, and
  the whole story is in its module docs.

What has *not* been tried is battery operation: both boards have only been run
over USB, where the Badger's GP10 holds up a rail the USB supply is feeding
anyway. See [the README's pin tables](../README.md#pins) for what every pin on
both boards does.
