//! The linker's search path.
//!
//! `memory.x` has to be on it, and it has to get there from `OUT_DIR`: the
//! linker runs from the directory cargo invokes it in, which is not the one
//! holding this file.

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
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
