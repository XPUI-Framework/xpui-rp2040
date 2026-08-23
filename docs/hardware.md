# Working on the RP2040 firmware

Everything past a first flash: what the keys do, why this crate is its own
workspace, where the memory goes, and which pin is which.

[`../README.md`](../README.md) is the front page — what you need, how to build,
how to flash.

## Using it

Five buttons, mapped by meaning rather than by position:

| Key | Badger | Tufty | The board says it sends |
|---|---|---|---|
| `a` | GP12 | GP7 | `Back` — **ignored on the root screen**, see below |
| `b` | GP13 | GP8 | `Confirm` — opens a row, commits a dialog |
| `c` | GP14 | GP9 | nothing |
| `Up` | GP15 | GP22 | `Up` |
| `Dn` | GP11 | GP6 | `Down` |

**The right-hand column is not written here.** `Buttons::new` looks each pin's
key up in the board it was handed — `pimoroni::BADGER_2040` or
`pimoroni::TUFTY_2040` — by the name in the left column, and takes what it sends
from there. This firmware states only which GPIO each switch is on, which is
the one thing a board cannot describe. Why `a` goes back and `c` is left bare
is written where the decision is, beside
[`BADGE_FOOTER`](https://github.com/XPUI-Framework/xpui-boards/blob/main/pimoroni/src/lib.rs). The hint bar above the
keys is painted from `BADGE_ROW` in that same file, and
`a_boards_keys_match_the_row_it_paints` holds the two together for every job
the bar has a word for — which is why giving `c` one is an edit to both.

Give `c` a job by editing that file — the entry in `BADGE_FOOTER`, its twin in
`BADGE_ROW`, and the line pinning it in
`the_badges_keys_send_what_the_firmware_wires`. This firmware picks it up
untouched. Both badges share that footer, so both gain the key.

Each key says what it resolved to on the way up, so a name the board does not
carry — the Inky Frame's are `A` to `E` — shows as `None` beside the name
that produced it rather than as a switch that feels broken. What no line can
show is a pin behind the wrong name: both sides agree, and only a thumb on the
board disagrees.

**Back is delivered everywhere; the root simply refuses to finish.** This
firmware calls `App::keep_root()`, so `Button::Back` reaches the screen as it
always does — a screen may claim it, an open value cancels with it — and only
the last of its three meanings, finishing the screen, is declined at the root.

Withholding the key instead is what this firmware used to do, and it took the
other two meanings with it: a screen could not dismiss its own picker, and a
value opened on a root screen could be committed but never cancelled. The
reason for the guard was real — finishing the last screen empties the stack,
ends the frame loop and parks the board, which from the outside is
indistinguishable from a crash — but the fix belongs where the stack is.

Each is debounced over four samples of a 10 ms loop and reported as an *edge*,
because `xpui`'s input is edge-based: a button reported as held on every frame
would re-fire whatever it is on.

On the Badger, expect a press to take about a second to appear. That is the
UC8151 refreshing, not the firmware thinking — `LUT::Fast` is already the
quickest waveform that leaves text crisp, and the loop only repaints when
`App::render_if_dirty` says something changed.

## Its own workspace

This crate is **excluded** from the repository's workspace, and that is what
makes it ordinary code you can open and read.

A workspace member is built for the host by `cargo clippy --workspace` and
`cargo test --workspace`, and an RP2040 HAL does not compile for a laptop. The
way out used to be a `device` cfg every item sat behind, so that off the board
the crate was empty — which also meant no test could reach it and an editor
showed nothing. Three mutations to the button mapping at once passed every
check in the repository.

Standing alone, its dependencies are unconditional and there is no cfg to
reason about. Point an editor at this directory and it works.

The cost is a lock file and a `target/` of its own, and that `-p xpui-rp2040`
no longer reaches it from the repository root. Everything the gate does to it
is by manifest path:

```bash
cargo fmt --check --manifest-path examples/rp2040/Cargo.toml
cargo clippy --release --manifest-path examples/rp2040/Cargo.toml \
  --all-targets --target thumbv6m-none-eabi -- -D warnings
```

`./build-and-test.sh` runs both. Breaking the firmware on purpose fails it —
that is checked, not assumed.

**Tests still do not run here**: a test harness needs libtest, which does not
exist for `thumbv6m-none-eabi`. What is testable belongs in a crate that
compiles for the host — which is why `PacedFill` now lives in
`xpui-embedded-graphics`, where a fake `DrawTarget` proves the one property it
exists for.

## Memory

`memory.x` declares 2 MB of flash — what a Badger has; a Tufty has 8 MB and is
happy with less being claimed — and the RP2040's 256 kB striped RAM bank.

The heap is 64 kB, in `src/runtime.rs`, with the reasoning written next to it.
It holds the leaked backend (which on the Badger carries the panel's own
4,736-byte framebuffer), the stack of live screens, and the view tree `body()`
rebuilds each frame. There is room to raise it — the whole allocated footprint
of a release build is:

| | Flash | RAM (`.data` + `.bss`) |
|---|---|---|
| `badger2040` | 221 kB of 2 MB | 94 kB of 256 kB |
| `tufty2040` | 227 kB of 8 MB | 67 kB of 256 kB |

Of the Badger's 94 kB, 64 kB is the heap and **29 kB is the embassy task pool**
— a `static` sized from the frame loop's future, which holds the panel driver
by value while `run` hands it on. The Tufty's is 1 kB. On the Badger that pool
is the largest thing after the heap and the first place to look if RAM runs
short; note it is six times the 4,736-byte framebuffer it carries, so shrinking
the driver saves more than its own size.

The rest of RAM is the stack, which grows down from the top.

**Most of that flash is type.** Around 100 kB of it is the two extra typefaces
the gallery registers — Courier and Century — which are bitmaps in `.rodata`
and cost nothing in RAM. They are reachable from nothing but
`gallery::fonts::FAMILIES`, so shortening that list to `&[&HELVETICA]` drops
them from the binary and takes it to about 55% of its size. Worth knowing
before porting this to a part
with less room; see [`examples/gallery/src/fonts.rs`](https://github.com/XPUI-Framework/xpui-gallery/blob/main/gallery/src/fonts.rs)
for the measured comparison.

These are measured from the allocated sections of a release ELF, not from the
file on disk — an ELF carries debug information the board never sees.

## Pins

Taken from Pimoroni's own board headers, not guessed. The key names are the
board's, because `Buttons::new` looks each one up by exactly that string.

**Badger 2040** — buttons `Dn` GP11, `a` GP12, `b` GP13, `c` GP14, `Up` GP15;
SPI0 clock GP18 and data GP19 (the panel never answers, so MISO is unused);
panel chip select GP17, data/command GP20, reset GP21, busy GP26; 3V3 enable
GP10, held high for as long as the firmware runs because on battery that pin
*is* the rail.

**Tufty 2040** — buttons `Dn` GP6, `a` GP7, `b` GP8, `c` GP9, `Up` GP22; panel
chip select GP10, data/command GP11, write GP12, read GP13; data bus DB0–DB7
on GP14–GP21 in order; backlight GP2, raised only after the first clear so the
controller's power-on noise is never lit. GP27 is the battery-sense reference
enable rather than a panel supply, and is left alone.
