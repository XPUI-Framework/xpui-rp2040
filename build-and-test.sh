#!/usr/bin/env bash
#
# Everything CI checks, in one command. The checks themselves are in `xtask/`,
# in Rust, where the text handling has tests; this only starts them.
#
# `--target` is named because `.cargo/config.toml` here makes the board the
# default for everything below this directory, and the gate runs on the host.
#
#   ./build-and-test.sh          check everything
#   ./build-and-test.sh fix      format in place first
#   ./build-and-test.sh all      the above, plus linking both firmware images

set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"
exec cargo run --quiet --manifest-path xtask/Cargo.toml \
  --target "$(rustc -vV | awk '/^host:/{print $2}')" -- "${1:-check}"
