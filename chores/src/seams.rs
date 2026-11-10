//! `cargo chores seams`: the dependency rules that keep each part of the
//! engine in its place, checked on the graph `cargo metadata` resolves
//! for the browser (wasm32-unknown-unknown) and again for this machine:
//!
//! 1. Only `wasm-bridge` reaches a browser crate (`wasm-bindgen`,
//!    `js-sys`, `web-sys`, `serde-wasm-bindgen`, `wasm-bindgen-futures`)
//!    or `getrandom` with its JavaScript feature. Normal and build
//!    dependencies count; test-only ones do not.
//! 2. `wasm-bridge` never reaches the tools: `command-line`, `chores`,
//!    `clap` or `rayon` (the browser has no threads).
//! 3. Nothing anywhere reaches a network crate, test-only dependencies
//!    included.

use crate::{Outcome, cargo};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

const BRIDGE: &str = "wasm-bridge";
const BROWSER: &[&str] = &[
    "wasm-bindgen",
    "js-sys",
    "web-sys",
    "serde-wasm-bindgen",
    "wasm-bindgen-futures",
];
const GETRANDOM_BROWSER_FEATURES: &[&str] = &["js", "wasm_js"];
const TOOLS: &[&str] = &["command-line", "chores", "clap", "rayon", "rayon-core"];
const NETWORK: &[&str] = &[
    "reqwest",
    "hyper",
    "hyper-util",
    "h2",
    "h3",
    "ureq",
    "curl",
    "curl-sys",
    "isahc",
    "surf",
    "attohttpc",
    "tungstenite",
    "tokio-tungstenite",
    "websocket",
    "tokio",
    "async-std",
    "smol",
    "mio",
    "socket2",
    "native-tls",
    "openssl",
    "openssl-sys",
    "rustls",
    "quinn",
];

/// The browser build's target platform.
const BROWSER_TARGET: &str = "wasm32-unknown-unknown";

pub fn run(root: &Path) -> Outcome {
    let mut found = Vec::new();
    let mut members = 0;
    for platform in [BROWSER_TARGET.to_string(), host_platform()?] {
        let metadata = metadata(root, &platform)?;
        members = metadata.workspace_members.len();
        found.extend(
            problems(&metadata)
                .into_iter()
                .map(|problem| format!("[{platform}] {problem}")),
        );
    }
    if found.is_empty() {
        println!("seams hold across {members} workspace crates, in the browser and natively");
        Ok(())
    } else {
        Err(format!(
            "{} seam(s) broken:\n  {}",
            found.len(),
            found.join("\n  ")
        ))
    }
}

/// The resolved dependency graph as cargo builds it for `platform`.
fn metadata(root: &Path, platform: &str) -> Result<Metadata, String> {
    let output = cargo()
        .current_dir(root)
        .args([
            "metadata",
            "--format-version",
            "1",
            "--filter-platform",
            platform,
        ])
        .output()
        .map_err(|error| format!("cannot run cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("cannot read cargo metadata: {error}"))
}

/// This machine's target triple, as rustc reports it.
fn host_platform() -> Result<String, String> {
    let output = std::process::Command::new("rustc")
        .arg("-vV")
        .output()
        .map_err(|error| format!("cannot run rustc: {error}"))?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .map(str::to_string)
        .ok_or_else(|| "rustc -vV did not name the host".to_string())
}

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
    resolve: Resolve,
}

#[derive(Deserialize)]
struct Package {
    id: String,
    name: String,
}

#[derive(Deserialize)]
struct Resolve {
    nodes: Vec<Node>,
}

#[derive(Deserialize)]
struct Node {
    id: String,
    deps: Vec<NodeDep>,
    features: Vec<String>,
}

#[derive(Deserialize)]
struct NodeDep {
    pkg: String,
    dep_kinds: Vec<DepKind>,
}

#[derive(Deserialize)]
struct DepKind {
    /// `null` for normal, `"build"` or `"dev"`.
    kind: Option<String>,
}

/// Every broken rule, each with the chain of crates that breaks it.
fn problems(metadata: &Metadata) -> Vec<String> {
    let names: BTreeMap<&str, &str> = metadata
        .packages
        .iter()
        .map(|package| (package.id.as_str(), package.name.as_str()))
        .collect();
    let nodes: BTreeMap<&str, &Node> = metadata
        .resolve
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();
    let name = |id: &str| names.get(id).copied().unwrap_or(id).to_string();

    let mut problems = Vec::new();
    for member in &metadata.workspace_members {
        let member_name = name(member);
        let shipped = reachable(&nodes, member, false);
        let everything = reachable(&nodes, member, true);
        for (id, path) in &shipped {
            let crate_name = name(id);
            let browser = BROWSER.contains(&crate_name.as_str())
                || (crate_name == "getrandom"
                    && nodes.get(id.as_str()).is_some_and(|node| {
                        node.features
                            .iter()
                            .any(|feature| GETRANDOM_BROWSER_FEATURES.contains(&feature.as_str()))
                    }));
            if browser && member_name != BRIDGE {
                problems.push(format!(
                    "only {BRIDGE} may reach browser crates: {}",
                    chain(path, &name)
                ));
            }
            if member_name == BRIDGE && TOOLS.contains(&crate_name.as_str()) {
                problems.push(format!(
                    "{BRIDGE} must not reach tools: {}",
                    chain(path, &name)
                ));
            }
        }
        for (id, path) in &everything {
            if NETWORK.contains(&name(id).as_str()) {
                problems.push(format!("no network crates: {}", chain(path, &name)));
            }
        }
    }
    problems.sort();
    problems.dedup();
    problems
}

/// Every crate `start` reaches, each with the path that first reaches it.
/// Test-only edges are followed only when `with_dev` is set, and only out
/// of `start` itself: a dependency's own tests are never built.
fn reachable(
    nodes: &BTreeMap<&str, &Node>,
    start: &str,
    with_dev: bool,
) -> BTreeMap<String, Vec<String>> {
    let mut found: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut seen = BTreeSet::from([start.to_string()]);
    let mut queue = VecDeque::from([vec![start.to_string()]]);
    while let Some(path) = queue.pop_front() {
        let here = path.last().expect("a path is never empty");
        let Some(node) = nodes.get(here.as_str()) else {
            continue;
        };
        for dep in &node.deps {
            let normal_or_build = dep
                .dep_kinds
                .iter()
                .any(|kind| kind.kind.as_deref() != Some("dev"));
            let follows = normal_or_build || (with_dev && path.len() == 1);
            if !follows || !seen.insert(dep.pkg.clone()) {
                continue;
            }
            let mut next = path.clone();
            next.push(dep.pkg.clone());
            found.insert(dep.pkg.clone(), next.clone());
            queue.push_back(next);
        }
    }
    found
}

fn chain(path: &[String], name: &impl Fn(&str) -> String) -> String {
    path.iter()
        .map(|id| name(id))
        .collect::<Vec<_>>()
        .join(" -> ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A workspace of `darkroom`, `wasm-bridge` and `chores`, with extra
    /// edges added per test.
    fn metadata(extra: &[(&str, &str, Option<&str>)], getrandom_features: &[&str]) -> Metadata {
        let mut edges = vec![
            ("wasm-bridge", "darkroom", None),
            ("wasm-bridge", "wasm-bindgen", None),
            ("wasm-bridge", "wasm-bindgen-test", Some("dev")),
            ("chores", "clap", None),
        ];
        edges.extend_from_slice(extra);
        let crates = [
            "darkroom",
            "wasm-bridge",
            "chores",
            "wasm-bindgen",
            "wasm-bindgen-test",
            "clap",
            "rayon",
            "getrandom",
            "tokio",
            "proptest",
        ];
        let nodes: Vec<_> = crates
            .iter()
            .map(|id| {
                let deps: Vec<_> = edges
                    .iter()
                    .filter(|(from, _, _)| from == id)
                    .map(|(_, to, kind)| json!({ "pkg": to, "dep_kinds": [{ "kind": kind }] }))
                    .collect();
                let features: &[&str] = if *id == "getrandom" {
                    getrandom_features
                } else {
                    &[]
                };
                json!({ "id": id, "deps": deps, "features": features })
            })
            .collect();
        let packages: Vec<_> = crates
            .iter()
            .map(|id| json!({ "id": id, "name": id }))
            .collect();
        serde_json::from_value(json!({
            "packages": packages,
            "workspace_members": ["darkroom", "wasm-bridge", "chores"],
            "resolve": { "nodes": nodes },
        }))
        .expect("the test metadata is well formed")
    }

    #[test]
    fn the_engine_as_built_holds() {
        assert_eq!(problems(&metadata(&[], &[])), Vec::<String>::new());
    }

    #[test]
    fn an_engine_crate_may_not_reach_the_browser() {
        let found = problems(&metadata(&[("darkroom", "wasm-bindgen", None)], &[]));
        assert_eq!(
            found,
            ["only wasm-bridge may reach browser crates: darkroom -> wasm-bindgen"]
        );
    }

    #[test]
    fn getrandom_counts_as_a_browser_crate_only_with_its_javascript_feature() {
        let edge = [("darkroom", "getrandom", None)];
        assert!(problems(&metadata(&edge, &["std"])).is_empty());
        assert_eq!(problems(&metadata(&edge, &["wasm_js"])).len(), 1);
    }

    #[test]
    fn test_only_browser_crates_are_allowed() {
        assert!(problems(&metadata(&[("darkroom", "wasm-bindgen", Some("dev"))], &[])).is_empty());
    }

    #[test]
    fn the_bridge_may_not_reach_tools() {
        let found = problems(&metadata(&[("darkroom", "rayon", None)], &[]));
        assert_eq!(
            found,
            ["wasm-bridge must not reach tools: wasm-bridge -> darkroom -> rayon"]
        );
    }

    #[test]
    fn network_crates_are_refused_even_in_tests() {
        let found = problems(&metadata(&[("chores", "tokio", Some("dev"))], &[]));
        assert_eq!(found, ["no network crates: chores -> tokio"]);
    }

    #[test]
    fn a_dependency_s_own_test_crates_are_not_followed() {
        let edges = [
            ("darkroom", "proptest", Some("dev")),
            ("proptest", "tokio", Some("dev")),
        ];
        assert!(problems(&metadata(&edges, &[])).is_empty());
    }
}
