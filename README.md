# The gallery, on hardware

Two firmware binaries that flash [the gallery](../gallery/) to a Pimoroni
board. Same screens the simulator runs, same crate, no device-specific code in
them at all.

| Binary | Board | Panel | Driver |
|---|---|---|---|
| `badger2040` | [Badger 2040](https://shop.pimoroni.com/products/badger-2040) | 296x128 monochrome e-ink | UC8151 over SPI |
| `tufty2040` | [Tufty 2040](https://shop.pimoroni.com/products/tufty-2040) | 320x240 colour IPS LCD | ST7789v over an 8-bit parallel bus |

What differs between them is a `Board`, a `Palette`, and which pin is wired to
what. The frame loop, the button handling and the heap are shared, and the
screens are not touched.

## What you need

- One of the two boards. Nothing else is required to flash over USB — both
  appear as a mass-storage device when you hold **BOOTSEL** while plugging them
  in.
- Optional, for `cargo run` and a debugger: a
  [Raspberry Pi Debug Probe](https://shop.pimoroni.com/products/raspberry-pi-debug-probe)
  or a second Pico running picoprobe, wired to the SWD pads.
- The target, which `rust-toolchain.toml` already lists:

  ```bash
  rustup target add thumbv6m-none-eabi   # only if it is somehow missing
  ```

## Build

From the repository root, by manifest path:

```bash
cargo build --release --manifest-path examples/rp2040/Cargo.toml \
  --bin badger2040 --target thumbv6m-none-eabi
cargo build --release --manifest-path examples/rp2040/Cargo.toml \
  --bin tufty2040 --target thumbv6m-none-eabi
```

Or from this directory, where `.cargo/config.toml` already sets the target:

```bash
cargo build --release --bin badger2040
```

Either way the ELF lands in `examples/rp2040/target/thumbv6m-none-eabi/release/`.
This crate is its own workspace, so it has a `target/` of its own — see
[its own workspace](#its-own-workspace).

### The one warning you will see

```text
warning: the following packages contain code that will be rejected by a future
version of Rust: proc-macro-error2 v2.0.1
```

Not ours, and not fixable here. It arrives four levels down —
`embassy-rp` → `pio` → `pio-proc` → `proc-macro-error2` — where that crate
re-exports `proc_macro` in a way
[rust-lang/rust#127909](https://github.com/rust-lang/rust/issues/127909) is
phasing out. 2.0.1 is the newest published version and still has it, so there is
nothing to upgrade into.

It is a cargo future-incompatibility report about a dependency's own source,
not a warning about this workspace, and it fails no gate. It is also a
**build-time** proc-macro running on your laptop: nothing it affects is
compiled into what the board runs. It goes away when `proc-macro-error2`
publishes a fix or `embassy-rp` moves off `pio-proc`; the only way to force it
sooner is a `[patch]` onto a fork of somebody else's crate, which is a worse
thing to own than a warning.

## Flash

**Over USB, no probe.** Install the tool first — it is a compile, and the board
should not be sitting in bootloader mode while it runs:

```bash
cargo install elf2uf2-rs
```

Then hold BOOTSEL, plug the board in, wait for the `RPI-RP2` drive, and **from
the repository root**:

```bash
elf2uf2-rs -d examples/rp2040/target/thumbv6m-none-eabi/release/badger2040
```

`-d` converts and copies in one step; the board reboots into the firmware by
itself. Without `-d` you get a `.uf2` beside the ELF to drag across yourself.

**With a probe.** From this directory:

```bash
cargo install probe-rs-tools
cargo run --release --bin badger2040
```

`.cargo/config.toml` points the runner at `probe-rs run --chip RP2040`. This
crate's release profile keeps the symbol table — see the note on it in
`Cargo.toml` — so a probe session shows names, and the RTT log arrives without
any extra flag.

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
key up in the board it was handed — `Board::BADGER_2040` or
`Board::TUFTY_2040` — by the name in the left column, and takes what it sends
from there. This firmware states only which GPIO each switch is on, which is
the one thing a board cannot describe. Why `a` goes back and `c` is left bare
is written where the decision is, beside
[`BADGE_FOOTER`](../../crates/boards/src/pimoroni.rs). The hint bar above the
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
with less room; see [`examples/gallery/src/fonts.rs`](../gallery/src/fonts.rs)
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

## What has and has not been run

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

`cargo run --release --bin badger2040` shows it, with no extra flag. That took
a fix: `probe-rs` finds the RTT control block by looking its symbol up in the
ELF, and the release profile used to `strip` the whole symbol table, so the log
never appeared and the board looked like it had printed nothing. The profile
now strips debug info and keeps the symbols.

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

Still unverified:

- **Battery operation.** Both boards have been run over USB only. The Badger's
  GP10 3V3 enable is held high for the firmware's lifetime, which matters only
  on battery and has not been tried there.
- **Anything about the Tufty's colour rendering** beyond ink and background.
  The framework paints in two colours and the panel does 65,536.

If a panel comes up inverted, the `Palette` is the wrong way round rather than
the firmware being broken. See `Palette::INK_IS_ON` / `INK_IS_OFF`.
