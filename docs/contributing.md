# Contributing to `xpui-rp2040`

## Building it

`rust-toolchain.toml` pins the toolchain and `thumbv6m-none-eabi`, the only
target this crate compiles for; `rustup` installs both on the first cargo
call. Every dependency on a sibling is fetched from its repository on `main`.

```bash
cargo build --release --bin badger2040     # .cargo/config.toml sets the target
./build-and-test.sh                        # everything CI checks
./build-and-test.sh all                    # the above, plus linking both images
```

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
**this repository holds three workspaces rather than one**: the firmware,
`docs-test/`, which compiles the tutorial and the reference as doctests, and
`xtask/`, the gate. `.cargo/config.toml` makes `thumbv6m-none-eabi` the default
target for everything below the root, `docs-test/` and `xtask/` included, so
neither names a triple: the gate passes the host triple whenever it builds
them. It also reaches all three by name:

```bash
cargo fmt --manifest-path Cargo.toml --all --check
cargo fmt --manifest-path docs-test/Cargo.toml --all --check
cargo fmt --manifest-path xtask/Cargo.toml --all --check
cargo clippy --release --all-targets --target thumbv6m-none-eabi -- -D warnings
```

`./build-and-test.sh` runs those and the two host-side lints beside them.
Breaking the firmware on purpose fails it — that is checked, not assumed.
**A `cargo fmt --check` or `cargo clippy` run at the root has checked one of
the three**, and leaves `docs-test/` and `xtask/` unformatted and unlinted.

**Tests still do not run here**: a test harness needs libtest, which does not
exist for `thumbv6m-none-eabi`. What is testable belongs in a crate that
compiles for the host — which is why `PacedFill` lives in
`xpui-embedded-graphics`, where a fake `DrawTarget` proves the one property it
exists for.

## The gate

A change is not finished until `./build-and-test.sh` passes. It is the same
command CI runs, so a green run locally means what a green tick means there.
The checks are listed in [`AGENTS.md`](../AGENTS.md) and implemented in
[`xtask/`](../xtask/); `./build-and-test.sh fix` formats in place first.
`all` additionally links both firmware images, which CI does not; run it
before pushing a change to the loop, the buttons or the runtime.

What bites here — no atomic compare-and-swap on Cortex-M0+, key labels that
resolve a real pin — is in `AGENTS.md`'s `## Style that bites here`, once.
That every pin in `ButtonPins` must be pulled down is written on the type, in
[`src/buttons.rs`](../src/buttons.rs).

## The review

Five steps, in order, none skipped:

1. The gate passes, with the real exit code read.
2. The [code-reviewer](../.claude/agents/code-reviewer.md) agent reviews the
   change — every finding resolved, not noted.
3. The [docs-reviewer](../.claude/agents/docs-reviewer.md) agent reviews the
   prose, last: it runs every command a document gives and resolves every
   snippet against the API.
4. The author flashes a board and presses all five keys. That is their step;
   hardware is not an agent's to sign off.
5. They say commit.

## Commits

The subject says what was done — imperative, under fifty characters, one
concern. The body says what changed and why, in under about ten lines,
carrying the fact that is not in the diff. Nothing about how the bug was
found. No self-attribution.

## Working across the repositories

This crate depends on four siblings through `git` dependencies on `main`, and
nothing depends on it. Before pushing a change that reaches into one of them,
run the umbrella:

```bash
for d in ../xpui*/; do git -C "$d" fetch --quiet --all; done
cd ../xpui-dev && ./build-and-test.sh cross
```

It builds every crate from the sibling checkouts on disk and says which one
broke. `cross` is that repository's gate, not this one's — run it from there,
not here. The fetch first, because its link check resolves every
`github.com/XPUI-Framework/…` URL against each sibling's `origin/main`, and a
stale remote is a stale answer. `xpui`'s [`docs/orientation.md`](https://github.com/XPUI-Framework/xpui-framework/blob/main/docs/orientation.md)
describes the layout it expects.
