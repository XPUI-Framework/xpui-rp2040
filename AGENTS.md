# `xpui-rp2040`

## What this is, and what it may not become

The gallery as firmware for two Pimoroni RP2040 boards: `badger2040` drives
296x128 of e-ink through a UC8151, `tufty2040` a 320x240 LCD through an
ST7789v. The crate supplies the three things a bare-metal host has to bring
itself — a heap, somewhere for a panic to go, and the frame loop that turns
GPIO edges into button presses and repaints only when the framework asks —
and two binaries that differ in a `Board`, a `Palette` and a pin map.

**Nothing here is a screen, a component or a board description.** The screens
are `xpui-gallery`'s, the boards are `xpui-boards-pimoroni`'s, the drawing is
`xpui-embedded-graphics`'s; this crate wires them and owns the pins. It
compiles for `thumbv6m-none-eabi` and nothing else: there is no host build
of the firmware, on purpose, which is why the tutorial's doctests and the
gate live in workspaces of their own.

## The gate

```bash
./build-and-test.sh          # everything below
./build-and-test.sh all      # the above, plus the line after the `+`
./build-and-test.sh fix      # the same as check, formatting in place first
```

```text
format · file sizes · crates are tested · READMEs warn · prose is compiled · documented paths resolve · rustdoc links resolve · the reference mirrors rustdoc · documented commands resolve · lint · the prose compiles · the gate's own tests · the nested clippy configs agree · README sections · AGENTS.md · published crates deny missing_docs · comment blocks · comment narration
+ both firmware images link
```

A last stage, `the gate is documented`, compares this list to what ran. Run
it before saying a change is done, and read the real exit code; run `all`
before pushing anything the linker could reject.

## What only this repository checks

- **`the gate's own tests`** and **`the nested clippy configs agree`** — the
  two nested workspaces are checked from here, and their `clippy.toml` must
  match the root's.
- **`both firmware images link`**, under `all` only: the one check that
  reaches the linker script, `memory.x` and both binaries.
- **`lint` runs three times**: `cargo clippy` for `thumbv6m-none-eabi`, the
  only build the firmware has, and then `docs-test/` and `xtask/` on the
  host, because a cargo command at the root reaches one workspace of three.
- **`the prose compiles`** — `docs-test/`'s doctests, run for the host
  triple, because the firmware crate cannot run a doctest on a laptop.
  `xpui-esp32` has a stage of the same name; the host triple is this one's.
- **`the reference mirrors rustdoc`** reads the board's rustdoc, under
  `target/thumbv6m-none-eabi/doc/xpui_rp2040`: a host run documents nothing
  here. With no output there it fails rather than skips.
- **`published crates deny missing_docs`** runs in every repository; what is
  particular here is its answer, `no publishable crates`, which is permanent.
  `#![deny(missing_docs)]` is on anyway.

## Style that bites here

- **Three workspaces.** `.cargo/config.toml` retargets everything below the
  root at the board; a cargo command run at the root has touched one of
  three. The gate reaches all of them.
- **Cortex-M0+ has no atomic compare-and-swap.** Load and store only; never
  `swap`, `fetch_or` or `compare_exchange`.
- **`no_std`, `alloc` from a 64 KiB first-fit heap.** No `format!` on the
  frame path. The boot log and the panic handler pay for `core::fmt`; the
  loop must not.
- **The key labels are what the hardware sends**: a pin is resolved by
  looking its label up in the bezel, and only a thumb on the board finds a
  pin behind the wrong name.
- **`present` runs only when the framework painted.** On the Badger it is
  most of a second; a loop presenting every frame leaves the panel
  permanently mid-refresh.
- **`strip = "debuginfo"`, never `true`**: `probe-rs` finds the RTT block by
  its symbol.
- **Every `pub` item is documented.**
- **A file under `src/` is at most 400 lines.**

## Where the documentation lives, and what proves each piece

| Document | Proven by |
|---|---|
| [`README.md`](README.md), [`docs-test/README.md`](docs-test/README.md) | their paths and commands resolve; neither carries a `rust` fence |
| [`docs/README.md`](docs/README.md) | its paths resolve; the README-heading check exempts it, because it is the index of `docs/`, not a front page |
| [`docs/tutorial.md`](docs/tutorial.md) | every `rust` fence is a doctest in `docs-test/`, mounted by `docs-test/src/lib.rs`; the boot log it quotes is checked by hand on a board, since no stage reads a `text` fence |
| [`docs/reference.md`](docs/reference.md) | `the reference mirrors rustdoc`, against the board's rustdoc; every `rust` fence is a doctest in `docs-test/`, mounted by `docs-test/src/lib.rs` |
| [`docs/hardware.md`](docs/hardware.md) | its paths resolve; its measurements are taken by hand from a release ELF, since no stage recomputes them |
| [`docs/contributing.md`](docs/contributing.md) | every path and command it gives resolves; the umbrella command is `xpui-dev`'s |
| `AGENTS.md` | the stage list above is compared to what the gate runs, in both modes |
| every `///` and `//!` | `rustdoc links resolve`, and the two comment checks |

## Git

Never stage, never commit, never push without being asked, each time. The
index is the reviewer's queue; leave new work unstaged. No self-attribution
in a commit message. Never rewrite a commit that exists; a correction is a new
commit. The rules that apply to all ten repositories, and the five review
steps, are in [`xpui`'s `docs/orientation.md`](https://github.com/XPUI-Framework/xpui-framework/blob/main/docs/orientation.md);
how a change is built and reviewed here is in
[`docs/contributing.md`](docs/contributing.md).
