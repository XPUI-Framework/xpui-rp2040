# `xpui-rp2040-docs`

> ⚠️ **Under heavy development.** Not production-ready. The API can break
> without notice. Use at your own risk.

A crate with no code, whose only job is to compile this repository's tutorial.

[`docs/tutorial.md`](../docs/tutorial.md) teaches somebody with a Badger 2040
in their hand, so it lives beside the firmware. But **rustdoc runs a snippet on
the host**, and nothing in the firmware crate builds for a laptop — an RP2040
HAL cannot. So the crate that compiles the prose cannot be the crate the prose
is about.

`src/lib.rs` is one `#[doc = include_str!]` and nothing else. Every ```rust
block in the tutorial is a doctest here.

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

Its own workspace, for the same reason — a member of the firmware's workspace
would inherit the board target.

## License

MIT — see [LICENSE](../LICENSE). Copyright (c) 2026 Thiago Holanda.
