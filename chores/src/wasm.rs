//! `cargo chores wasm [--dev|--release]`: builds the engine for the
//! browser. Checks the wasm-bindgen versions match, builds `wasm-bridge`
//! for wasm32 (with +simd128, from `.cargo/config.toml`), then runs
//! `wasm-bindgen --target web` into `app/src/engine-room/wasm-build/`,
//! which is git-ignored and rewritten whole each time.

use crate::{Outcome, cargo, versions};
use std::path::{Path, PathBuf};
use std::process::Command;

const OUT_DIR: &str = "app/src/engine-room/wasm-build";

pub fn run(root: &Path, dev: bool) -> Outcome {
    versions::run(root)?;

    let (profile, profile_dir) = if dev {
        ("wasm-dev", "wasm-dev")
    } else {
        ("release", "release")
    };
    let built = cargo()
        .current_dir(root)
        .args([
            "build",
            "-p",
            "wasm-bridge",
            "--target",
            "wasm32-unknown-unknown",
        ])
        .args(["--profile", profile])
        .status()
        .map_err(|error| format!("cannot run cargo: {error}"))?;
    if !built.success() {
        return Err("the wasm build failed".to_string());
    }

    let wasm = target_dir(root)
        .join("wasm32-unknown-unknown")
        .join(profile_dir)
        .join("wasm_bridge.wasm");
    let out = root.join(OUT_DIR);
    if out.exists() {
        std::fs::remove_dir_all(&out)
            .map_err(|error| format!("cannot clear {OUT_DIR}: {error}"))?;
    }
    let bound = Command::new("wasm-bindgen")
        .args(["--target", "web", "--out-dir"])
        .arg(&out)
        .arg(&wasm)
        .status()
        .map_err(|error| format!("cannot run wasm-bindgen: {error}"))?;
    if !bound.success() {
        return Err("wasm-bindgen failed".to_string());
    }
    println!("engine ({profile}) written to {OUT_DIR}/");
    Ok(())
}

/// Where cargo puts build output.
fn target_dir(root: &Path) -> PathBuf {
    std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target"))
}
