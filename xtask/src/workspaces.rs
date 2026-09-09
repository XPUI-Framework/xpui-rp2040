//! The two host workspaces beside the firmware: `docs-test/`, which compiles
//! the tutorial, and `xtask/`, which is this gate.
//!
//! Both are their own workspaces because `.cargo/config.toml` at the root
//! targets the board for everything below it, and every command here names
//! the host triple for the same reason.

use crate::cargo;

/// Rustdoc, for the board and for the two workspaces beside the root.
///
/// The board run is the one this repository could not do any other way: a
/// doc comment behind `cfg(target_os = "none")` is not parsed by a host
/// rustdoc, and this crate is nothing but such code.
pub fn rustdoc_links() -> Result<String, String> {
    let mut notes = Vec::new();
    if cargo::target_installed("thumbv6m-none-eabi") {
        cargo::rustdoc(&["--release", "--target", "thumbv6m-none-eabi"])?;
        notes.push("thumbv6m-none-eabi".to_string());
    } else {
        notes.push("thumbv6m-none-eabi SKIPPED — rustup target add thumbv6m-none-eabi".into());
    }
    let host = cargo::host_triple();
    for manifest in ["docs-test/Cargo.toml", "xtask/Cargo.toml"] {
        cargo::rustdoc(&["--manifest-path", manifest, "--target", &host])?;
        notes.push(format!("{manifest} on the host"));
    }
    Ok(notes.join(", "))
}

/// The tutorial, compiled.
///
/// `--target` is named explicitly: `.cargo/config.toml` above this directory
/// sets the board as the default target for everything below it, and a doctest
/// has to run somewhere it can run.
pub fn docs_test() -> Result<String, String> {
    cargo::cargo(&[
        "test",
        "--manifest-path",
        "docs-test/Cargo.toml",
        "--doc",
        "--target",
        &cargo::host_triple(),
    ])
}

/// The checks' own unit tests.
///
/// This repository has no host workspace to test — an RP2040 HAL does not
/// compile for a laptop — so `cargo test` is not in the list above, and the
/// gate's own tests would never run here without this line.
pub fn gate_tests() -> Result<String, String> {
    cargo::cargo(&[
        "test",
        "--manifest-path",
        "xtask/Cargo.toml",
        "--target",
        &cargo::host_triple(),
    ])
}

/// The nested workspaces lint under the same rules as the firmware.
///
/// Clippy reads the `clippy.toml` nearest the workspace root, and `docs-test/`
/// and `xtask/` are their own workspaces — so without a copy each would lint
/// under cargo's defaults rather than this organisation's `msrv`. `xpui-dev`
/// compares the root file across every repository and cannot see these two:
/// its list is one path, `clippy.toml`, and these are two directories down.
pub fn nested_clippy_agrees() -> Result<String, String> {
    let root = std::fs::read_to_string("clippy.toml").map_err(|e| format!("  clippy.toml: {e}"))?;
    let wanted: Vec<&str> = root
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    if wanted.is_empty() {
        return Err("clippy.toml sets nothing, so this would compare nothing".into());
    }

    let mut drifted = Vec::new();
    let mut checked = Vec::new();
    for nested in ["docs-test/clippy.toml", "xtask/clippy.toml"] {
        let Ok(text) = std::fs::read_to_string(nested) else {
            drifted.push(format!(
                "  {nested} is missing, so that workspace lints under cargo's defaults"
            ));
            continue;
        };
        let here: Vec<&str> = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect();
        if here == wanted {
            checked.push(nested);
        } else {
            drifted.push(format!("  {nested} sets different values from clippy.toml"));
        }
    }
    if drifted.is_empty() {
        Ok(format!(
            "{} setting(s), in {}",
            wanted.len(),
            checked.join(" and ")
        ))
    } else {
        Err(format!(
            "{}\n\nEach nested workspace needs the root file's values, or it lints\n\
             under different rules from the firmware one directory up.",
            drifted.join("\n")
        ))
    }
}
