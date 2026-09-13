# Working on the RP2040 firmware

Everything past a first flash: what the keys do, why this crate is its own
workspace, where the memory goes, which pin is which, what the release profile
keeps, why one loop serves both boards, and what running it proved.

[`../README.md`](../README.md) is the front page — how to build, how to flash,
what you need.

## The keys

Five buttons, mapped by meaning rather than by position:

| Key | Badger | Tufty | The board says it sends |
|---|---|---|---|
| `a` | GP12 | GP7 | `Back` — **never finishes the root screen**, see below |
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

Withholding the key instead takes the other two meanings with it: a screen
cannot dismiss its own picker, and a value opened on a root screen can be
committed but never cancelled. The reason to reach for the guard is real —
finishing the last screen empties the stack, ends the frame loop and parks
the board, which from the outside is indistinguishable from a crash — but the
fix belongs where the stack is.

Each is debounced over four samples of a 10 ms loop and reported as an *edge*,
because `xpui`'s input is edge-based: a button reported as held on every frame
would re-fire whatever it is on.

On the Badger, expect a press to take about a second to appear. That is the
UC8151 refreshing, not the firmware thinking — `LUT::Fast` is already the
quickest waveform that leaves text crisp, and the loop only repaints when
`App::render_if_dirty` says something changed.

## Its own workspace

This crate is the workspace root, and `docs-test/` and `xtask/` are workspaces
of their own rather than members of it. That is what makes the firmware
ordinary code you can open and read.

A member is built for the host by `cargo clippy --workspace` and `cargo test
--workspace`, and an RP2040 HAL does not compile for a laptop. The alternative
is a `device` cfg every item sits behind, so that off the board the crate is
empty — which also means no test can reach it and an editor shows nothing.
Three mutations to the button mapping at once would pass every check in the
repository.

Standing alone, its dependencies are unconditional and there is no cfg to
reason about. Point an editor at this directory and it works.

The cost is a lock file and a `target/` of its own — and, less obviously, that
**this repository holds three workspaces rather than one**. `.cargo/config.toml`
makes `thumbv6m-none-eabi` the default target for everything below the root,
`docs-test/` and `xtask/` included, so neither names a triple: the gate passes
the host triple whenever it builds them. It also reaches all three by name:

```bash
cargo fmt --manifest-path Cargo.toml --all --check
cargo fmt --manifest-path docs-test/Cargo.toml --all --check
cargo fmt --manifest-path xtask/Cargo.toml --all --check
cargo clippy --release --all-targets --target thumbv6m-none-eabi -- -D warnings
```

`./build-and-test.sh` runs those and the two host-side lints beside them.
Breaking the firmware on purpose fails it — that is checked, not assumed.

Running only the two lines a reader would guess at — `cargo fmt --check` and
the clippy above — leaves `docs-test/` and `xtask/` unformatted and unlinted.

**Tests still do not run here**: a test harness needs libtest, which does not
exist for `thumbv6m-none-eabi`. What is testable belongs in a crate that
compiles for the host — which is why `PacedFill` lives in
`xpui-embedded-graphics`, where a fake `DrawTarget` proves the one property it
exists for.

## Memory

`memory.x` declares 2 MiB of flash — what a Badger has; a Tufty has 8 MiB and
is happy with less being claimed — and the RP2040's 256 KiB striped RAM bank.

The heap is 64 KiB, in `src/runtime.rs`, with the reasoning written next to it.
It holds the leaked backend (which on the Badger carries the panel's own
4,736-byte framebuffer), the stack of live screens, and the view tree `body()`
rebuilds each frame. There is room to raise it — the whole allocated footprint
of a release build is:

| | Flash | RAM (`.data` + `.bss`) |
|---|---|---|
| `badger2040` | 225 KiB of 2 MiB | 94 KiB of 256 KiB |
| `tufty2040` | 230 KiB of 8 MiB | 66 KiB of 256 KiB |

Of the Badger's 94 KiB, 64 KiB is the heap and **28 KiB is the embassy task
pool** — a `static` sized from the frame loop's future, which holds the panel
driver by value while `run` hands it on. The Tufty's is 624 bytes. On the
Badger that pool is the largest thing after the heap and the first place to
look if RAM runs short; note it is six times the 4,736-byte framebuffer it
carries, so shrinking the driver saves more than its own size.

The rest of RAM is the stack, which grows down from the top.

**Most of that flash is type.** Around 100 KiB of it is the two extra typefaces
the gallery registers — Courier and Century — which are bitmaps in `.rodata`
and cost nothing in RAM. They are reachable from nothing but
`gallery::fonts::FAMILIES`, so shortening that list to `&[&HELVETICA]` drops
them from the binary and takes it to about 55% of its size. Worth knowing
before porting this to a part
with less room; see [`gallery/src/fonts.rs`](https://github.com/XPUI-Framework/xpui-gallery/blob/main/gallery/src/fonts.rs)
for the figure, and that repository's `docs/design.md` for the measurement.

These are measured from the allocated sections of a release ELF, not from the
file on disk — an ELF carries debug information the board never sees. Flash is
`.vector_table` + `.text` + `.rodata` + `.data` + `.boot2`; RAM is `.data` +
`.bss`; the task pool is the size of its `POOL` symbol. Measured on
2026-09-13; re-measure rather than trust them.

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

## The release profile

`strip = "debuginfo"` rather than `true`. `probe-rs run` locates the RTT
control block by looking its symbol up in the ELF, and `strip = true` deletes
the whole symbol table, so the boot log never appears and the failure looks
like a firmware that printed nothing. DWARF still goes.

It costs nothing on the part. The symbol table is in no loadable segment, so
what is flashed is byte-identical either way, measured on both boards. The ELF
on disk grows by about 100 KiB, which is host disk and not flash.

## One loop for both boards

`run` is `run_async` with a blocking present wrapped as an async one that
never suspends. Keeping one loop rather than two that would drift costs
+736 bytes on the Badger and +800 on the Tufty, most of it the display-loan
machinery rather than the wrapper.

Both boards go through the blocking one because neither has an async flush to
wait on: `mipidsi` writes straight through, and `uc8151`'s published release
spins on the BUSY pin. Its `asynch` module exists on git and has never been
published — the last release was 2023 — so moving the Badger to it would mean
a git dependency and an `embedded-hal` 1.0 migration for a driver nobody has
cut a release of since. `run_async` is there for when that changes, and for
any DMA-backed panel today.

## What running it proved

**Both boards have been run**, over a debug probe, and the firmware reports
what it finds on the way up:

```text
xpui: Badger 2040 296x128, panel 296x128
xpui: key a sends Some(Back)
xpui: key b sends Some(Confirm)
xpui: key c sends None
xpui: key Up sends Some(Up)
xpui: key Dn sends Some(Down)
xpui: first frame up, heap 5308 of 65536 used
```

A mismatch between the first two sizes is a driver configured a quarter turn
out, which lays out plausibly and puts the screen in a corner of the glass. It
costs one line to say so and an afternoon to find otherwise.

The five key lines are the same idea: each is what the board answered for a
name this firmware wires. They catch a name it does not carry — that key
resolves to `None` and is silent — and they cannot catch a pin behind the wrong
name, which is what pressing all five is for. Both boards have been pressed
through all five, and `a` and `b` were checked as the pair most worth getting
backwards.

`cargo run --release --bin badger2040` shows it, with no extra flag — which
is what [the release profile](#the-release-profile) keeping the symbol table
buys.

Three faults came out of that first session and are fixed:

| | |
|---|---|
| Auto-repeat counted a panel refresh as a held key | one tap of Down walked the selection several rows. `Runtime` no longer credits a gap it could not see through |
| `Back` on the root screen parked the board | it emptied the screen stack, ended the loop, and looked exactly like a crash. `App::keep_root()` now declines the pop; the key is still delivered, so a screen can claim it and an open value still cancels with it |
| `mipidsi` outran the ST7789 over the parallel bus | its repeated-pixel shortcut pulses the write strobe at ~30 ns against a 66 ns minimum. Any pixel whose two bytes match takes it — 256 of them, ink and background among them — so fills came out as noise while text stayed crisp. See `PacedFill` in `xpui-embedded-graphics` |

**The heap figure is measured, not reasoned** — 5,308 bytes on the Badger and
592 on the Tufty, of 65,536. One sample, of the root menu at boot: it says the
reservation in `src/runtime.rs` is generous, not that it is generous under every
screen. A screen that buffers an image has not been tried.

Two things a probe cannot reach, for whoever gets there next:

- **Battery operation.** Both boards have been run over USB only, so the
  Badger's GP10 3V3 enable — [the pin that *is* the rail](#pins) on battery —
  has never been exercised where it matters.
- **Anything about the Tufty's colour rendering** beyond ink and background.
  The framework paints in two colours and the panel does 65,536.

If a panel comes up inverted, the `Palette` is the wrong way round rather than
the firmware being broken. See `Palette::INK_IS_ON` / `INK_IS_OFF`.
