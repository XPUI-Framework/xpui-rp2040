//! Nothing but a home for the tutorial's doctests.
//!
//! `../docs/tutorial.md`'s and `../docs/reference.md`'s snippets are compiled
//! and run from here, because
//! the firmware they teach cannot compile anything on a laptop: every item in
//! it is bare metal, and rustdoc runs snippets on the host.
//!
//! A snippet that stops matching the API fails this crate's `cargo test`. That
//! is the whole of its job — it exports nothing and is never flashed.
//!
//! It has to be run with the host named, because `.cargo/config.toml` at the
//! repository root sets `build.target` to the board and cargo reads a parent
//! config from every directory below it:
//!
//! ```bash
//! cargo test --manifest-path docs-test/Cargo.toml --doc \
//!   --target "$(rustc -vV | awk '/^host:/{print $2}')"
//! ```
//!
//! Naming the host in a committed config instead would pin one machine's
//! triple, and CI does not run on that machine.

#[cfg(doctest)]
#[doc = include_str!("../../docs/tutorial.md")]
mod tutorial {}

#[cfg(doctest)]
#[doc = include_str!("../../docs/reference.md")]
mod reference {}
