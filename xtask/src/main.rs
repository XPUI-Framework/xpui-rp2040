//! The gate for `xpui-rp2040`.
//!
//! Everything CI checks, in one command, and **only what this repository has
//! to check**. It is firmware for a Cortex-M0+, so there is no
//! host workspace to test and one bare-metal lint that is not optional. The
//! prose is compiled by `docs-test/`, a second workspace that names the host
//! triple because `.cargo/config.toml` here targets the board.
//!
//! ```text
//! ./build-and-test.sh          check everything
//! ./build-and-test.sh fix      format in place first
//! ./build-and-test.sh all      the above, plus linking both firmware images
//! ```
//!
//! Each repository in the organisation has its own copy of this shape, holding
//! its own list. **This file is the part that is meant to differ**; the modules
//! under it are byte-identical, and `shared_files_agree` in `xpui-dev` hashes
//! all seven across the nine, so a fix to the fence scanner cannot land in one
//! repository and not the rest.
//!
//! A check written and never listed below is a dead function, which clippy
//! fails the build over. That is what a hand-written "is every check
//! dispatched?" check used to do, and it does it better.

mod cargo;
mod commands;
mod docs;
mod faults;
mod fences;
mod paths;
mod prose;
mod tree;

use std::process::ExitCode;

/// Files under a `src/` may not exceed this. A ratchet, not a law of nature:
/// raising it is a decision to argue for in a commit message, never a way to
/// land a file.
const LINE_LIMIT: usize = 400;

/// Crates with no tests, and why. The reason prints on every run so it is
/// re-read rather than accumulated — and an exemption for a crate that has
/// since grown tests fails, rather than sitting there as a comment nobody
/// removes.
const UNTESTED: [(&str, &str); 2] = [
    (
        ".",
        "the firmware; test = false, and a HAL cannot compile for a laptop",
    ),
    (
        "docs-test",
        "a doctest mount; its only content is this repository's tutorial",
    ),
];

/// Fence languages this repository's prose is written in.
///
/// The list exists so that ` ```rustt ` is an error rather than a shrug: an
/// unknown language silently compiles nothing, and a typo is the likeliest
/// way for a Rust block to stop being checked.
const KNOWN_LANGUAGES: [&str; 19] = [
    "text", "bash", "sh", "shell", "console", "cpp", "c", "toml", "yaml", "yml", "json", "ini",
    "diff", "ascii", "mermaid", "markdown", "md", "python", "cmake",
];

/// Documents whose ```rust is illustrative rather than compilable.
const NOT_COMPILED: [&str; 0] = [];

/// Pages that are not a repository's front door and carry no banner.
const NOT_A_FRONT_PAGE: [&str; 0] = [];

/// The one bare-metal target, and it is required. There is no host build here
/// to fall back on: an RP2040 HAL does not compile for a laptop, so this lint
/// is the only thing that reads this code before a board does.
const BARE_METAL: [(&str, bool); 1] = [("thumbv6m-none-eabi", true)];

/// The whole crate, because the whole crate is the firmware.
const LINT_CRATES: [&str; 1] = ["--all-targets"];

fn main() -> ExitCode {
    // Every path in every check is relative to the repository root, so the
    // gate answers the same from anywhere it is invoked.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask/..");
    std::env::set_current_dir(root).expect("the repository root");

    // A typo is not a check. The shell this replaced rejected an unknown
    // argument, and a gate that silently treats `fx` as `check` is a gate that
    // reports a pass for a run nobody asked for.
    let (fix, everything) = match std::env::args().nth(1).as_deref() {
        None | Some("check") => (false, false),
        Some("fix") => (true, false),
        Some("all") => (false, true),
        Some(other) => {
            eprintln!("unknown argument `{other}`\nusage: ./build-and-test.sh [check|fix|all]");
            return ExitCode::from(2);
        }
    };
    let mut failed = 0;

    let mut gate: Vec<(&str, Box<dyn Fn() -> Result<String, String>>)> = vec![
        (
            "format",
            Box::new(move || {
                // Three workspaces, not one. `docs-test/` and `xtask/` are
                // their own because `.cargo/config.toml` here retargets
                // everything below the root at the board — so `--all` from the
                // root reaches neither, and for a while nothing formatted or
                // linted the gate itself.
                for manifest in ["Cargo.toml", "docs-test/Cargo.toml", "xtask/Cargo.toml"] {
                    let mut arguments = vec!["fmt", "--manifest-path", manifest, "--all"];
                    if !fix {
                        arguments.push("--check");
                    }
                    cargo::cargo(&arguments)?;
                }
                Ok("the firmware, docs-test and xtask".into())
            }),
        ),
        ("file sizes", Box::new(|| tree::file_sizes(LINE_LIMIT))),
        (
            "crates are tested",
            Box::new(|| tree::crates_are_tested(&UNTESTED)),
        ),
        (
            "READMEs warn",
            Box::new(|| tree::readmes_warn(&NOT_A_FRONT_PAGE)),
        ),
        (
            "prose is compiled",
            Box::new(|| prose::is_compiled(&NOT_COMPILED, &KNOWN_LANGUAGES)),
        ),
        ("documented paths resolve", Box::new(docs::doc_paths)),
        ("rustdoc links resolve", Box::new(rustdoc_links)),
        (
            "documented commands resolve",
            Box::new(|| commands::resolve(&cargo::packages(), &[])),
        ),
        ("lint", Box::new(lint)),
        ("the prose compiles", Box::new(docs_test)),
        ("the gate's own tests", Box::new(gate_tests)),
        (
            "the nested clippy configs agree",
            Box::new(nested_clippy_agrees),
        ),
    ];

    // `all` is what a laptop runs before a board is flashed, and what CI runs
    // in a job provisioned for it. It is separate from `check` because these
    // stages need a toolchain or a build system that a quick run should not
    // demand.
    if everything {
        gate.extend::<Vec<(&str, Box<dyn Fn() -> Result<String, String>>)>>(vec![(
            "both firmware images link",
            Box::new(firmware_links),
        )]);
    }

    for (name, check) in gate.drain(..) {
        println!("\n==> {name}");
        match check() {
            Ok(note) if note.is_empty() => println!("    ok"),
            Ok(note) => println!("    {}", note.replace('\n', "\n    ")),
            Err(why) => {
                println!("{why}");
                eprintln!("FAILED: {name}");
                failed += 1;
            }
        }
    }

    if failed == 0 {
        if everything {
            println!("\nEverything passed.");
        } else {
            println!("\nChecks passed. `./build-and-test.sh all` also links both images.");
        }
        ExitCode::SUCCESS
    } else {
        eprintln!("\n{failed} check(s) failed.");
        ExitCode::FAILURE
    }
}

/// Clippy for the board, and for the workspace that compiles this
/// repository's prose.
fn lint() -> Result<String, String> {
    let mut notes = Vec::new();
    bare_metal(&mut notes)?;
    cargo::cargo(&[
        "clippy",
        "--manifest-path",
        "docs-test/Cargo.toml",
        "--all-targets",
        "--target",
        &cargo::host_triple(),
        "--",
        "-D",
        "warnings",
    ])?;
    notes.push("docs-test on the host".into());
    cargo::cargo(&[
        "clippy",
        "--manifest-path",
        "xtask/Cargo.toml",
        "--all-targets",
        "--target",
        &cargo::host_triple(),
        "--",
        "-D",
        "warnings",
    ])?;
    notes.push("xtask on the host".into());
    Ok(notes.join(", "))
}

/// The checks' own unit tests.
///
/// This repository has no host workspace to test — an RP2040 HAL does not
/// compile for a laptop — so `cargo test` is not in the list above, and the
/// gate's own tests would never run here without this line.
fn gate_tests() -> Result<String, String> {
    cargo::cargo(&[
        "test",
        "--manifest-path",
        "xtask/Cargo.toml",
        "--target",
        &cargo::host_triple(),
    ])
}

/// The tutorial, compiled.
///
/// `--target` is named explicitly: `.cargo/config.toml` above this directory
/// sets the board as the default target for everything below it, and a doctest
/// has to run somewhere it can run.
fn docs_test() -> Result<String, String> {
    cargo::cargo(&[
        "test",
        "--manifest-path",
        "docs-test/Cargo.toml",
        "--doc",
        "--target",
        &cargo::host_triple(),
    ])
}

/// Clippy on each bare-metal target, with warnings as errors.
///
/// The host build never parses code behind `cfg(target_os = "none")` — no
/// allocator, no panic handler — so these are the only gates that reach it
/// before a firmware build does. Neither target has atomic compare-and-swap:
/// load and store only, never `swap`, `fetch_or` or `compare_exchange`. The
/// second is a second architecture rather than a stricter one.
fn bare_metal(notes: &mut Vec<String>) -> Result<(), String> {
    {
        for (triple, required) in BARE_METAL {
            {
                if !cargo::target_installed(triple) {
                    {
                        if required {
                            {
                                return Err(format!(
                                    "{triple} is not installed, and it is the only gate that reaches\n\
                     this repository's no_std paths. `rustup target add {triple}`"
                                ));
                            }
                        }
                        notes.push(format!("{triple} SKIPPED — rustup target add {triple}"));
                        continue;
                    }
                }
                let mut arguments = vec!["clippy", "--release"];
                arguments.extend_from_slice(&LINT_CRATES);
                arguments.extend_from_slice(&["--target", triple, "--", "-D", "warnings"]);
                cargo::cargo(&arguments)?;
                notes.push(triple.to_string());
            }
        }
        Ok(())
    }
}

/// Both firmware images link.
///
/// Not part of `check`: it is a release build of two binaries, and a quick run
/// should not pay for it. It is the last thing before a board is flashed, and
/// the only stage that proves the linker script and the HAL agree.
fn firmware_links() -> Result<String, String> {
    if !cargo::target_installed("thumbv6m-none-eabi") {
        return Ok("skipped: rustup target add thumbv6m-none-eabi".into());
    }
    for board in ["badger2040", "tufty2040"] {
        cargo::cargo(&[
            "build",
            "--release",
            "--bin",
            board,
            "--target",
            "thumbv6m-none-eabi",
        ])?;
    }
    Ok("badger2040, tufty2040".into())
}

/// Rustdoc, for the board and for the two workspaces beside the root.
///
/// The board run is the one this repository could not do any other way: a
/// doc comment behind `cfg(target_os = "none")` is not parsed by a host
/// rustdoc, and this crate is nothing but such code.
fn rustdoc_links() -> Result<String, String> {
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

/// The nested workspaces lint under the same rules as the firmware.
///
/// Clippy reads the `clippy.toml` nearest the workspace root, and `docs-test/`
/// and `xtask/` are their own workspaces — so without a copy each would lint
/// under cargo's defaults rather than this organisation's `msrv`. `xpui-dev`
/// compares the root file across every repository and cannot see these two:
/// its list is one path, `clippy.toml`, and these are two directories down.
///
/// Which makes this the only thing that reads them, and the copies had a
/// comment claiming otherwise.
fn nested_clippy_agrees() -> Result<String, String> {
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
