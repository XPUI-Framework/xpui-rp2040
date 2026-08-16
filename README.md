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

From the workspace root:

```bash
cargo build --release -p xpui-rp2040 --bin badger2040 --target thumbv6m-none-eabi
cargo build --release -p xpui-rp2040 --bin tufty2040  --target thumbv6m-none-eabi
```

Or from this directory, where `.cargo/config.toml` already sets the target:

```bash
cargo build --release --bin badger2040
```

Either way the ELF lands in `target/thumbv6m-none-eabi/release/`, relative to
the workspace root.

### The one warning you will see

```text
warning: the following packages contain code that will be rejected by a future
version of Rust: proc-macro-error2 v2.0.1
```

Not ours, and not fixable here. It arrives four levels down —
`embassy-rp` → `pio` → `pio-proc` → `proc-macro-error2` — and 2.0.1 is the
newest published version, so there is nothing to upgrade to. It is a cargo
future-incompatibility report about a dependency's own source, not a warning
about this workspace, and it fails no gate. It goes away when `embassy-rp`
moves off `pio-proc`.

## Flash

**Over USB, no probe.** Hold BOOTSEL, plug the board in, wait for the `RPI-RP2`
drive to appear, then:

```bash
cargo install elf2uf2-rs
elf2uf2-rs -d target/thumbv6m-none-eabi/release/badger2040
```

`-d` converts and copies in one step; the board reboots into the firmware by
itself. Without `-d` you get a `.uf2` beside the ELF to drag across yourself.

**With a probe.** From this directory:

```bash
cargo install probe-rs-tools
cargo run --release --bin badger2040
```

`.cargo/config.toml` points the runner at `probe-rs run --chip RP2040`. The
workspace release profile strips symbols, so a probe session shows addresses
rather than names — build without `--release` if you need to read a backtrace.

## Using it

Five buttons, mapped by meaning rather than by position:

| Button | Badger | Tufty | Does |
|---|---|---|---|
| B / centre | GP13 | GP8 | `Confirm` — opens a row, commits a dialog |
| A / left | GP12 | GP7 | `Back` |
| C / right | GP14 | GP9 | `Right` — increments a stepper or a slider |
| Up | GP15 | GP22 | `Up` |
| Down | GP11 | GP6 | `Down` |

Each is debounced over four samples of a 10 ms loop and reported as an *edge*,
because `xpui`'s input is edge-based: a button reported as held on every frame
would re-fire whatever it is on.

On the Badger, expect a press to take about a second to appear. That is the
UC8151 refreshing, not the firmware thinking — `LUT::Fast` is already the
quickest waveform that leaves text crisp, and the loop only repaints when
`App::render_if_dirty` says something changed.

## Why this crate builds for the host as an empty one

The workspace's gates — `cargo clippy --workspace`, `cargo test --workspace` —
build *every* member for the machine they run on, and an RP2040 HAL does not
compile for a laptop. So the dependencies here sit behind
`[target.'cfg(all(target_arch = "arm", target_os = "none"))'.dependencies]`,
`build.rs` sets a matching `device` cfg, and every item in `src/` is gated on
it. Off the device this crate is empty, and the host gates pass with it as a
full member rather than an excluded directory nobody builds.

The cost is that `cargo clippy --workspace` never lints this code. Lint it by
building it for the target it is for, which is the only place it exists:

```bash
cargo clippy --release -p xpui-rp2040 --all-targets --target thumbv6m-none-eabi -- -D warnings
```

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
| `badger2040` | 56 kB of 2 MB | 74 kB of 256 kB, 64 kB of it the heap |
| `tufty2040` | 61 kB of 8 MB | 65 kB of 256 kB, 64 kB of it the heap |

The rest of RAM is the stack, which grows down from the top.

## Pins

Taken from Pimoroni's own board headers, not guessed.

**Badger 2040** — buttons down GP11, A GP12, B GP13, C GP14, up GP15; SPI0
clock GP18 and data GP19 (the panel never answers, so MISO is unused); panel
chip select GP17, data/command GP20, reset GP21, busy GP26; 3V3 enable GP10,
held high for as long as the firmware runs because on battery that pin *is* the
rail.

**Tufty 2040** — buttons down GP6, A GP7, B GP8, C GP9, up GP22; panel chip
select GP10, data/command GP11, write GP12, read GP13; data bus DB0–DB7 on
GP14–GP21 in order; backlight GP2, raised only after the first clear so the
controller's power-on noise is never lit. GP27 is the battery-sense reference
enable rather than a panel supply, and is left alone.

## What has not been verified

**Neither firmware has been run on a board.** Both link, and both lay out
correctly for the RP2040 boot ROM — `.boot2` at `0x10000000`, the vector table
at `0x10000100`, `.data` with a flash LMA and a SRAM VMA — which is what
`elf2uf2-rs` and `probe-rs` need. Correct linkage is not correct behaviour.

Three pin decisions deliberately differ from Pimoroni's reference code. Each is
reasoned, none is confirmed on hardware, and each is a one-line change if a
board disagrees:

| Pin | Here | Reference | Why |
|---|---|---|---|
| Badger **GP10** | driven high for the firmware's lifetime | driven high at init, low to power down | On battery this pin *is* the 3V3 rail. The reference plausibly gets away with the difference because USB feeds the rail anyway. |
| Tufty **GP27** | left alone | declared, never constructed | Pimoroni's `battery.py` shows it is the ADC reference-divider enable, driven *low* when idle — nothing to do with the LCD. |
| Tufty **GP10** | driven low explicitly | declared, never driven | Chip select. The reference presumably relies on the RP2040's default pull-down. Explicit is safer, but unconfirmed. |

**Tufty orientation and colour inversion** — `display_size(240, 320)` with
`Deg270` and `Inverted` — are copied from the reference. mipidsi reports
320 × 240 after that rotation, which matches `Board::TUFTY_2040`, but that the
image is the right way up is untested.

**The 64 kB heap is reasoned, not measured.** The justification is in
`src/runtime.rs`. Measure it before trusting it on a bigger screen set.

If you flash one of these, the polarity is the first thing to check: an
inverted panel means the `Palette` is the wrong way round, not that the
firmware is broken. See `Palette::INK_IS_ON` / `INK_IS_OFF`.
