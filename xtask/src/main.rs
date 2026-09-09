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
//! all ten across the nine, so a fix to the fence scanner cannot land in one
//! repository and not the rest.
//!
//! A check written and never listed below is a dead function, which clippy
//! fails the build over.

mod agents;
mod cargo;
mod commands;
mod comments;
mod docs;
mod faults;
mod fences;
mod paths;
mod prose;
mod readme;
mod tree;
mod workspaces;

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

/// The root README's headings, in order. Empty until this repository's front
/// page is brought to the standard; then the eight.
const README_ORDER: &[&str] = &[];
const README_OPTIONAL: &[&str] = &["Which crate you want", "Requirements"];
const NESTED_ORDER: &[&str] = &[
    "Using it",
    "Requirements",
    "Checking it",
    "Where next",
    "License",
];
const NESTED_OPTIONAL: &[&str] = &["Requirements", "Where next"];

/// `AGENTS.md` exists and `CLAUDE.md` is a symlink to it.
const AGENTS_FILE: bool = false;

/// Every publishable crate denies `missing_docs`. `true` here says so for
/// none: nothing in this repository is published.
const DOCUMENTED: bool = true;

/// How long a comment may be. `None` is not adopted.
const COMMENT_CAPS: Option<comments::Caps> = Some(comments::Caps {
    doc: 15,
    header: 15,
    run: 10,
});
/// No comment is about the past.
const NARRATION_CHECKED: bool = true;
/// Which files the two comment checks read. `None` is every tracked source,
/// manifest and C++ file outside `tests/`.
const COMMENT_SCOPE: Option<&str> = None;

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

    // A typo is not a check: a gate that silently treats `fx` as `check`
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
                // everything below the root at the board, so `--all` from the
                // root reaches neither.
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
        ("rustdoc links resolve", Box::new(workspaces::rustdoc_links)),
        (
            "documented commands resolve",
            Box::new(|| commands::resolve(&cargo::packages(), &[])),
        ),
        ("lint", Box::new(lint)),
        ("the prose compiles", Box::new(workspaces::docs_test)),
        ("the gate's own tests", Box::new(workspaces::gate_tests)),
        (
            "the nested clippy configs agree",
            Box::new(workspaces::nested_clippy_agrees),
        ),
        (
            "README sections",
            Box::new(|| {
                readme::readme_sections(
                    README_ORDER,
                    README_OPTIONAL,
                    NESTED_ORDER,
                    NESTED_OPTIONAL,
                    &NOT_A_FRONT_PAGE,
                )
            }),
        ),
        (
            "AGENTS.md",
            Box::new(|| agents::agents_file_exists(AGENTS_FILE)),
        ),
        (
            "published crates deny missing_docs",
            Box::new(|| tree::published_crates_deny_missing_docs(DOCUMENTED)),
        ),
        (
            "comment blocks",
            Box::new(|| comments::comment_blocks(COMMENT_CAPS, COMMENT_SCOPE)),
        ),
        (
            "comment narration",
            Box::new(|| comments::comment_narration(NARRATION_CHECKED, COMMENT_SCOPE)),
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

    // Last, after every insert and extend, owning the names: a closure in
    // the vector cannot borrow the vector.
    let names: Vec<String> = gate.iter().map(|(n, _)| n.to_string()).collect();
    gate.push((
        "the gate is documented",
        Box::new(move || agents::agents_documents_the_gate(&names)),
    ));

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
/// Clippy on the one bare-metal target, with warnings as errors.
///
/// Nothing here compiles for a host, so this is the only gate that reads
/// the code before a board does. The target has no atomic compare-and-swap:
/// load and store only, never `swap`, `fetch_or` or `compare_exchange`.
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
