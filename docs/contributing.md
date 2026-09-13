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

## Three workspaces

`.cargo/config.toml` retargets everything below the root at the board, so the
two crates that must build for a laptop are their own workspaces:
`docs-test/`, which compiles the tutorial as doctests, and `xtask/`, the
gate. The gate reaches all three. **A `cargo fmt --check` or `cargo clippy`
run at the root has checked one of the three**; run `./build-and-test.sh`,
which does not forget the other two.

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
