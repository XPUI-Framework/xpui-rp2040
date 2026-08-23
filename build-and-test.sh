#!/usr/bin/env bash

# Everything CI checks in this repository, in one command.
#
#   ./build-and-test.sh          format, lint, the tutorial, and the prose
#   ./build-and-test.sh check    the same thing; the name CI uses
#   ./build-and-test.sh all      the above, plus linking both firmware images
#   ./build-and-test.sh fix      format Rust in place first
#
# **Half of what runs is in `bin/gate-common.sh`**, of which every repository
# in the organisation carries a byte-identical copy. This file is what this
# repository configures, what only it checks, and the order they run in.
# `xpui-dev` compares the nine copies and runs all nine gates.
#
# **Nothing here compiles for a laptop.** An RP2040 HAL cannot, so
# `HOST_WORKSPACE=0` and the host clippy, test, doctest and rustdoc runs all
# say so rather than failing. What replaces them is one bare-metal lint and
# `docs-test/`, a host-target crate whose only job is to compile this
# repository's tutorial.
#
# **A flashed board is not checked by anything below**, and cannot be. Both
# boards have been run over a debug probe; that is a person with hardware.

set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${PROJECT_DIR}"

# The firmware at the root, and the crate that compiles the prose beside it.
SOURCE_ROOTS=(.)

# `docs-test` is its own workspace — it has to be, since it targets the host
# and everything above it targets the board — so `cargo fmt` at the root never
# reaches it.
EXTRA_MANIFESTS=(docs-test/Cargo.toml)

# **No `?` on this one.** Elsewhere a trailing `?` skips a bare-metal target
# that is not installed rather than failing a gate somebody cannot fix without
# a download — which is safe there, because a host run still compiles the code.
# Here there is no host run: with the target absent and the `?` present, this
# gate ran zero clippy invocations, compiled none of `src/`, printed one
# `skipped:` line and said "Checks passed."
HOST_WORKSPACE=0
LINT_TARGETS=("thumbv6m-none-eabi")
LINT_TARGET_CRATES=(--all-targets)

# Two crates here have no tests, and both are deliberate.
#
# The firmware sets `test = false` on every target: a test harness needs
# libtest, which does not exist for `thumbv6m-none-eabi`, so the only thing
# `--all-targets` could do is fail. What its glue does — panel init, refresh
# pacing, key debouncing — is unreachable from a laptop by construction. The
# UI it draws is `xpui-gallery`'s, snapshotted seventy ways there.
#
# `docs-test` is one `include_str!` and nothing else. Its whole job is to be a
# host-target crate that compiles this repository's prose, and `test_extra`
# runs exactly that.
UNTESTED_CRATES=(
  ".:the firmware; test = false, and a HAL cannot compile for a laptop"
  "docs-test:a doctest mount; its only content is this repository's tutorial"
)

. bin/gate-common.sh

# ---------------------------------------------------------------------------
# What only this repository checks.
# ---------------------------------------------------------------------------

# The host name, asked of rustc rather than written down.
#
# `.cargo/config.toml` sets `[build] target` to the board, and cargo reads a
# parent config from every directory below it — so a `cargo test` in
# `docs-test` would target Cortex-M0+ and rustdoc cannot run a snippet there.
# Naming a triple in a committed file would pin one machine, so it is asked.
host_target() {
  rustc -vV | awk '/^host:/{print $2}'
}

lint_extra() {
  # `docs-test` is its own workspace, so the run above never reaches it. It is
  # one `include_str!` and a manifest, but a manifest that stops resolving is
  # the tutorial silently no longer being compiled.
  say "Clippy, the crate that compiles the tutorial"
  cargo clippy --manifest-path docs-test/Cargo.toml --all-targets \
    --target "$(host_target)" -- -D warnings
}

test_extra() {
  # The tutorial, compiled. `--doc` because that is all this crate is for: its
  # `lib.rs` is an `include_str!` of `docs/tutorial.md` and nothing else.
  say "The tutorial's snippets compile"
  cargo test --manifest-path docs-test/Cargo.toml --doc --target "$(host_target)"
}

doc_link_extra() {
  say "Every doc link resolves on thumbv6m-none-eabi"
  if target_installed thumbv6m-none-eabi; then
    RUSTDOCFLAGS="-D warnings" cargo doc --release --no-deps --target thumbv6m-none-eabi
  else
    echo "    skipped: rustup target add thumbv6m-none-eabi"
  fi
}

# Both images, linked. Not run: flashing is a person with a board.
firmware_links() {
  say "Both firmware images link"
  if ! target_installed thumbv6m-none-eabi; then
    echo "    skipped: rustup target add thumbv6m-none-eabi"
    return 0
  fi
  cargo build --release --bin badger2040 --target thumbv6m-none-eabi
  cargo build --release --bin tufty2040 --target thumbv6m-none-eabi
}

# ---------------------------------------------------------------------------

gates() {
  file_sizes
  crates_are_tested
  every_check_runs
  readmes_warn
  prose_is_compiled
  doc_paths
  commands_resolve
  cpp_snippets_compile
  lint
  test_suite
  doc_tests
  doc_links
}

case "${1:-check}" in
  check)
    run_all "${FORMAT_CHECK[@]}"
    gates
    printf '\nChecks passed. "./build-and-test.sh all" also links both images.\n'
    ;;
  fix)
    run_all "${FORMAT_FIX[@]}"
    gates
    printf '\nFormatted and checked.\n'
    ;;
  all)
    run_all "${FORMAT_CHECK[@]}"
    gates
    firmware_links
    printf '\nEverything passed.\n'
    ;;
  *)
    echo "usage: ./build-and-test.sh [check|fix|all]" >&2
    exit 2
    ;;
esac
