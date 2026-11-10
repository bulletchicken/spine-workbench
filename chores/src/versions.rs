//! `cargo chores versions`: the wasm-bindgen crate in `Cargo.lock` and the
//! wasm-bindgen command-line tool must be the same version exactly, or the
//! JavaScript glue does not match the module it loads.

use crate::Outcome;
use std::path::Path;
use std::process::Command;

pub fn run(root: &Path) -> Outcome {
    let lock = std::fs::read_to_string(root.join("Cargo.lock"))
        .map_err(|error| format!("cannot read Cargo.lock: {error}"))?;
    let crate_version = locked_version(&lock, "wasm-bindgen")?;
    let tool_version = tool_version()?;
    if crate_version != tool_version {
        return Err(format!(
            "the wasm-bindgen crate is {crate_version} but the wasm-bindgen tool is \
             {tool_version}; install the matching tool with \
             `cargo install wasm-bindgen-cli --version ={crate_version} --locked`"
        ));
    }
    println!("wasm-bindgen {crate_version}: crate and tool match");
    Ok(())
}

/// The one version of `name` in a `Cargo.lock`.
fn locked_version(lock: &str, name: &str) -> Result<String, String> {
    let wanted = format!("name = \"{name}\"");
    let mut versions = Vec::new();
    let mut lines = lock.lines();
    while let Some(line) = lines.next() {
        if line.trim() != wanted {
            continue;
        }
        let version = lines
            .next()
            .and_then(|next| next.trim().strip_prefix("version = "))
            .map(|quoted| quoted.trim_matches('"').to_string())
            .ok_or_else(|| format!("Cargo.lock has no version line after `{wanted}`"))?;
        versions.push(version);
    }
    match versions.as_slice() {
        [one] => Ok(one.clone()),
        [] => Err(format!("Cargo.lock has no {name} crate")),
        many => Err(format!(
            "Cargo.lock has {} versions of {name}: {}",
            many.len(),
            many.join(", ")
        )),
    }
}

/// The version `wasm-bindgen --version` prints.
fn tool_version() -> Result<String, String> {
    let output = Command::new("wasm-bindgen")
        .arg("--version")
        .output()
        .map_err(|error| format!("cannot run the wasm-bindgen tool: {error}"))?;
    if !output.status.success() {
        return Err("`wasm-bindgen --version` failed".to_string());
    }
    let printed = String::from_utf8_lossy(&output.stdout);
    printed
        .split_whitespace()
        .nth(1)
        .map(str::to_string)
        .ok_or_else(|| format!("unexpected `wasm-bindgen --version` output: {printed}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOCK: &str = r#"
[[package]]
name = "js-sys"
version = "0.3.107"

[[package]]
name = "wasm-bindgen"
version = "0.2.130"
source = "registry+https://github.com/rust-lang/crates.io-index"

[[package]]
name = "wasm-bindgen-shared"
version = "0.2.130"
"#;

    #[test]
    fn the_locked_version_is_read_by_exact_name() {
        assert_eq!(
            locked_version(LOCK, "wasm-bindgen").as_deref(),
            Ok("0.2.130")
        );
        assert!(locked_version(LOCK, "web-sys").is_err());
    }

    #[test]
    fn two_locked_versions_are_refused() {
        let doubled =
            format!("{LOCK}\n[[package]]\nname = \"wasm-bindgen\"\nversion = \"0.2.129\"\n");
        assert!(locked_version(&doubled, "wasm-bindgen").is_err());
    }
}
