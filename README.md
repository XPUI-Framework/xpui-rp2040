# The gallery, on hardware

> ⚠️ **Under heavy development.** Not production-ready. The API can break
> without notice. Use at your own risk.

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

## Working on it

[`docs/hardware.md`](docs/hardware.md) is everything past a first flash:

| | |
|---|---|
| [Using it](docs/hardware.md#using-it) | what each key does on each board, and why Back is delivered even at the root |
| [Its own workspace](docs/hardware.md#its-own-workspace) | why this crate is excluded from the root one, and what that costs |
| [Memory](docs/hardware.md#memory) | where the framebuffer lives, what the heap is for, and how much is left |
| [Pins](docs/hardware.md#pins) | the wiring, per board |

[`docs/tutorial.md`](docs/tutorial.md) is the walk-through: a board is data, the
palette trap, and the panel driver seam. Its snippets are compiled — by
`docs-test/`, which exists because nothing in this crate builds for a laptop
and rustdoc runs snippets on the host:

```bash
cargo test --manifest-path docs-test/Cargo.toml --doc \
  --target "$(rustc -vV | awk '/^host:/{print $2}')"
```

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
