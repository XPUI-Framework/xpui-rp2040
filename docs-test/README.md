[![CI](https://github.com/XPUI-Framework/xpui-rp2040/actions/workflows/ci.yml/badge.svg)](https://github.com/XPUI-Framework/xpui-rp2040/actions/workflows/ci.yml) [![MIT](https://img.shields.io/badge/license-MIT-blue.svg)](../LICENSE)

# `xpui-rp2040-docs`

A crate with no code, whose only job is to compile this repository's tutorial.

## Using it

[`docs/tutorial.md`](../docs/tutorial.md) teaches somebody with a Badger 2040
in their hand, so it lives beside the firmware. But **rustdoc runs a snippet on
the host**, and nothing in the firmware crate builds for a laptop — an RP2040
HAL cannot. So the crate that compiles the prose cannot be the crate the prose
is about. `src/lib.rs` is one `#[doc = include_str!]` and nothing else, and
every Rust fence in the tutorial is a doctest here.

From the repository root, one directory up:

```bash
cargo test --manifest-path docs-test/Cargo.toml --doc \
  --target "$(rustc -vV | awk '/^host:/{print $2}')"
```

**The host triple is named explicitly**, and that is not decoration: the
repository's `.cargo/config.toml` sets `[build] target = "thumbv6m-none-eabi"`,
and cargo reads a parent config from every directory below it. Without the
flag this crate would try to build for the board, and rustdoc would have
nothing it could run. Naming a triple in a committed file would pin one
machine, so it is asked of `rustc` instead.

Its own workspace for a different reason: a member of the firmware's
workspace is built by `cargo test --workspace` at the root, which is a build
for the board, and the point of this crate is to be built for a laptop.

## Checking it

The gate is the repository's; `./build-and-test.sh` from the root runs this
crate's doctests as its `the prose compiles` stage.

## License

MIT — see [LICENSE](../LICENSE). Copyright (c) 2026 Thiago Holanda.
