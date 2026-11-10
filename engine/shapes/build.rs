//! Computes the shapes fingerprint: the SHA-256 of every source file of
//! this crate, read in name order. The wasm build and the generated
//! TypeScript both carry it, so a stale wasm build is caught at worker boot
//! instead of corrupting data.

// A build script runs on the developer's machine, not in the engine, so it
// may read files.
#![allow(clippy::disallowed_methods)]

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=src");
    let mut files = Vec::new();
    collect(Path::new("src"), &mut files);
    files.sort();

    let mut hasher = Sha256::new();
    for file in &files {
        let name = file.to_string_lossy().replace('\\', "/");
        let bytes = std::fs::read(file).expect("a shapes source file is readable");
        hasher.update((name.len() as u64).to_le_bytes());
        hasher.update(name.as_bytes());
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(&bytes);
    }
    let hex: String = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    println!("cargo:rustc-env=SHAPES_FINGERPRINT={hex}");
}

fn collect(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir).expect("the shapes source folder is readable");
    for entry in entries {
        let path = entry.expect("a folder entry is readable").path();
        if path.is_dir() {
            collect(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
}
