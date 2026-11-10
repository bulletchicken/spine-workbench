//! `cargo chores <task>`: the upkeep that keeps the repository honest. A
//! native tool, never shipped; one task per file.
//!
//! | Task | Does |
//! |---|---|
//! | `wasm [--dev\|--release]` | builds the engine for the browser into `app/src/engine-room/wasm-build/` |
//! | `shapes [--check]` | writes the TypeScript shapes, the case file's JSON Schema and the fingerprint; `--check` fails on drift |
//! | `seams` | fails if a crate reaches what its place in the engine forbids |
//! | `versions` | fails unless the wasm-bindgen crate and command-line tool versions match |
//! | `map [--check]` | fails if a path named in `MAP.md` does not exist |

// Chores are repository upkeep run on a developer's machine or in CI, not
// engine code: they read and write files, run cargo and wasm-bindgen, and
// read the environment cargo sets.
#![allow(clippy::disallowed_methods, clippy::disallowed_types)]
#![forbid(unsafe_code)]

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod map;
mod seams;
mod shapes;
mod versions;
mod wasm;

/// What a task returns: nothing on success, or what is wrong.
type Outcome = Result<(), String>;

#[derive(Parser)]
#[command(name = "cargo chores", about = "Repository upkeep tasks.")]
struct Cli {
    #[command(subcommand)]
    task: Task,
}

#[derive(Subcommand)]
enum Task {
    /// Build the engine for the browser: cargo build for wasm32, then
    /// wasm-bindgen into app/src/engine-room/wasm-build/.
    Wasm {
        /// The quicker wasm-dev profile (debug assertions on).
        #[arg(long, conflicts_with = "release")]
        dev: bool,
        /// The release profile (the default).
        #[arg(long)]
        release: bool,
    },
    /// Write the TypeScript shapes, the JSON Schema and the fingerprint.
    Shapes {
        /// Write nothing; fail if the committed files differ.
        #[arg(long)]
        check: bool,
    },
    /// Check the dependency rules between crates.
    Seams,
    /// Check the wasm-bindgen crate and command-line tool versions match.
    Versions,
    /// Check every path named in MAP.md exists.
    Map {
        /// Print only problems.
        #[arg(long)]
        check: bool,
    },
}

fn main() -> ExitCode {
    let root = match repository_root() {
        Ok(root) => root,
        Err(problem) => {
            eprintln!("cargo chores: {problem}");
            return ExitCode::FAILURE;
        }
    };
    let (name, outcome) = match Cli::parse().task {
        Task::Wasm { dev, .. } => ("wasm", wasm::run(&root, dev)),
        Task::Shapes { check } => ("shapes", shapes::run(&root, check)),
        Task::Seams => ("seams", seams::run(&root)),
        Task::Versions => ("versions", versions::run(&root)),
        Task::Map { check } => ("map", map::run(&root, check)),
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(problem) => {
            eprintln!("cargo chores {name}: {problem}");
            ExitCode::FAILURE
        }
    }
}

/// The repository root: the folder of the workspace's `Cargo.toml`, as
/// cargo finds it from the current folder when the task runs. Never the
/// folder the binary was built in, so a binary reused from a shared target
/// folder still works on the checkout it is run from.
fn repository_root() -> Result<PathBuf, String> {
    let output = cargo()
        .args(["locate-project", "--workspace", "--message-format", "plain"])
        .output()
        .map_err(|error| format!("could not run cargo locate-project: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo locate-project failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let manifest = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    manifest
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| format!("{} has no parent folder", manifest.display()))
}

/// The cargo that is running this task, so nested builds use the same
/// toolchain.
fn cargo() -> std::process::Command {
    std::process::Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
}
