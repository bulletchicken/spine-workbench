//! `cargo chores map [--check]`: every path named in `MAP.md` exists.
//!
//! A path is an inline code span or a link target, outside fenced code
//! blocks, that has no spaces, no wildcards or placeholders (`*`, `{`,
//! `<`), and either contains a `/`, ends in a known file extension, is a
//! dotfile name such as `.npmrc`, or is one of the root files with no
//! extension (`LICENSE`, `NOTICE`). A path ending in `/` must be a folder.
//! Paths are relative to the repository root.

use crate::Outcome;
use std::path::Path;

/// Files named without an extension that MAP.md lists.
const BARE_NAMES: &[&str] = &["LICENSE", "NOTICE"];

const EXTENSIONS: &[&str] = &[
    "rs", "ts", "tsx", "js", "json", "jsonl", "md", "toml", "yml", "yaml", "sh", "css", "html",
    "svg", "lock", "txt",
];

pub fn run(root: &Path, check: bool) -> Outcome {
    let text = std::fs::read_to_string(root.join("MAP.md"))
        .map_err(|error| format!("cannot read MAP.md: {error}"))?;
    let paths = named_paths(&text);
    if paths.is_empty() {
        return Err("MAP.md names no paths".to_string());
    }
    let missing: Vec<&String> = paths.iter().filter(|path| !exists(root, path)).collect();
    if !check {
        for path in &paths {
            let state = if missing.contains(&path) {
                "missing"
            } else {
                "ok"
            };
            println!("{state:>7}  {path}");
        }
    }
    if missing.is_empty() {
        println!("all {} paths in MAP.md exist", paths.len());
        Ok(())
    } else {
        let list: Vec<&str> = missing.iter().map(|path| path.as_str()).collect();
        Err(format!(
            "{} path(s) in MAP.md do not exist:\n  {}",
            missing.len(),
            list.join("\n  ")
        ))
    }
}

/// Every path the text names, in order of first mention, each once.
fn named_paths(text: &str) -> Vec<String> {
    let mut paths: Vec<String> = Vec::new();
    let mut fenced = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        let spans = line.split('`').skip(1).step_by(2);
        let links = line
            .split("](")
            .skip(1)
            .filter_map(|rest| rest.split(')').next());
        for candidate in spans.chain(links) {
            let candidate = candidate.split('#').next().unwrap_or_default();
            let candidate = candidate.strip_prefix("./").unwrap_or(candidate);
            if looks_like_path(candidate) && !paths.iter().any(|known| known == candidate) {
                paths.push(candidate.to_string());
            }
        }
    }
    paths
}

fn looks_like_path(candidate: &str) -> bool {
    if candidate.is_empty()
        || candidate.contains(char::is_whitespace)
        || candidate.contains("://")
        || candidate.contains(['*', '{', '<', '|'])
        || candidate.starts_with(['/', '-'])
        || candidate.starts_with("..")
    {
        return false;
    }
    // A dotfile or dot-folder: `.npmrc`, `.github/workflows/app.yml`.
    let dotted = candidate
        .strip_prefix('.')
        .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_alphabetic()));
    if candidate.starts_with('.') && !dotted {
        return false;
    }
    let extension = candidate
        .rsplit_once('.')
        .map(|(_, extension)| extension)
        .unwrap_or_default();
    dotted
        || candidate.contains('/')
        || EXTENSIONS.contains(&extension)
        || BARE_NAMES.contains(&candidate)
}

fn exists(root: &Path, path: &str) -> bool {
    let full = root.join(path);
    if path.ends_with('/') {
        full.is_dir()
    } else {
        full.exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_come_from_code_spans_and_links() {
        let text = "\
# Map
We spell it `centre`.
| Change | Open |
|---|---|
| the shapes | `engine/shapes/src/lib.rs`, [the schema](./docs/contracts/x.json#top) |
| the readme | `README.md` and `engine/` |
| the root | `LICENSE`, `.npmrc`, `.github/workflows/app.yml`, `../outside` |
Run `cargo chores map --check`; marks such as `supL`; `f64::sin`; `.5 mm`.
```
`engine/not-a-row.rs`
```
A step: `app/src/steps/<n>-<verb>/`, a glob `engine/*`, a site <https://example.org>.
";
        assert_eq!(
            named_paths(text),
            [
                "engine/shapes/src/lib.rs",
                "docs/contracts/x.json",
                "README.md",
                "engine/",
                "LICENSE",
                ".npmrc",
                ".github/workflows/app.yml",
            ]
        );
    }

    #[test]
    fn a_folder_path_must_be_a_folder() {
        // Cargo runs unit tests from the crate's own folder.
        let root = Path::new(".");
        assert!(exists(root, "src/"));
        assert!(!exists(root, "Cargo.toml/"));
        assert!(exists(root, "Cargo.toml"));
        assert!(!exists(root, "src/nowhere.rs"));
    }
}
