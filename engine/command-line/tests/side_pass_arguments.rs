//! Argument refusals happen before any input file is opened.
use std::process::Command;

#[test]
fn side_arguments_require_a_side_film_and_an_explicit_trained_facing() {
    for (args, expected) in [
        (
            vec!["pass", "--side-finder", "trained", "side=absent"],
            "pass-facing-required",
        ),
        (
            vec!["pass", "--side-facing", "left", "front=absent"],
            "side-specific flags require side=PATH",
        ),
        (
            vec!["pass", "--side-finder", "hand-built", "front=absent"],
            "side-specific flags require side=PATH",
        ),
        (
            vec!["pass", "--side-facing", "up", "side=absent"],
            "invalid value",
        ),
        (
            vec!["pass", "--side-finder", "other", "side=absent"],
            "invalid value",
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_spine"))
            .args(args)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains(expected));
    }
}
