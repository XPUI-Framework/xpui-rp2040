//! Two jobs, both of which only mean anything on the hardware.
//!
//! The first is the `device` cfg that every item in this crate sits behind. It
//! is on for bare-metal ARM and nowhere else, so the workspace's host gates —
//! which build every member for the machine they run on — see an empty crate
//! rather than an RP2040 HAL that cannot compile for a laptop.
//!
//! The second is the linker. `memory.x` has to be on the search path, and it
//! has to get there from `OUT_DIR`: in a workspace the linker runs from the
//! workspace root, where the crate's own directory is not looked in.

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    // Declared unconditionally, including on the host, or every `cfg(device)`
    // in the crate becomes an `unexpected_cfgs` warning — and warnings fail.
    println!("cargo::rustc-check-cfg=cfg(device)");

    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if arch != "arm" || os != "none" {
        return;
    }
    println!("cargo::rustc-cfg=device");

    let out = PathBuf::from(env::var_os("OUT_DIR").expect("cargo always sets OUT_DIR"));
    fs::write(out.join("memory.x"), include_bytes!("memory.x")).expect("OUT_DIR is writable");
    println!("cargo::rustc-link-search={}", out.display());
    println!("cargo::rerun-if-changed=memory.x");

    // `--nmagic` stops the linker page-aligning sections. The RP2040's flash
    // opens with a 256-byte second-stage bootloader at a fixed address, and
    // alignment padding pushes the vector table off where the ROM looks.
    println!("cargo::rustc-link-arg-bins=--nmagic");
    // cortex-m-rt's script first, then embassy-rp's, which adds `.boot2`.
    println!("cargo::rustc-link-arg-bins=-Tlink.x");
    println!("cargo::rustc-link-arg-bins=-Tlink-rp.x");
}
