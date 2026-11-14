//! Selects the optional side weights before compiling the engine.

// Build scripts run outside the engine and may inspect their own inputs.
#![allow(clippy::disallowed_methods)]
#![forbid(unsafe_code)]

use std::io::ErrorKind;
use std::path::Path;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(side_finder_weights)");
    for path in [
        "weights",
        "weights/on-this-machine",
        "weights/on-this-machine/side-finder.bin",
    ] {
        println!("cargo:rerun-if-changed={path}");
        match std::fs::symlink_metadata(Path::new(path)) {
            Ok(metadata) => {
                assert!(
                    !metadata.file_type().is_symlink(),
                    "a side weight input must not be a symlink: {path}"
                );
                if path.ends_with(".bin") {
                    assert!(
                        metadata.is_file(),
                        "the side weight packet must be a regular file"
                    );
                    println!("cargo:rustc-cfg=side_finder_weights");
                } else {
                    assert!(
                        metadata.is_dir(),
                        "a side weight parent must be a directory: {path}"
                    );
                }
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => panic!("cannot inspect side weight input {path}: {error}"),
        }
    }
}
