//! Tests the tag validator in `scripts/bump-libopenmpt.sh`.
//!
//! The validator is the only gate before the script checks out a submodule
//! tag, so a wrong pattern would reach the network with a bad name.

use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;

/// Returns the path to the bump script.
fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/bump-libopenmpt.sh")
}

/// Runs the script with one argument set.
fn run(args: &[&str]) -> Result<Output, String> {
    Command::new("bash")
        .arg(script())
        .args(args)
        .output()
        .map_err(|error| format!("cannot run the bump script: {error}"))
}

#[test]
fn accepts_a_release_tag_and_prints_its_fields() -> Result<(), String> {
    let cases = [
        ("libopenmpt-0.8.10", ["0", "8", "10"]),
        ("libopenmpt-1.0.0", ["1", "0", "0"]),
        ("libopenmpt-12.34.567", ["12", "34", "567"]),
    ];
    for (tag, wanted) in cases {
        let output = run(&["--check-tag", tag])?;
        if !output.status.success() {
            return Err(format!(
                "{tag} must be accepted, but the script gave {:?}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        let fields: Vec<&str> = std::str::from_utf8(&output.stdout)
            .map_err(|error| format!("the output for {tag} is not text: {error}"))?
            .lines()
            .collect();
        if fields != wanted {
            return Err(format!("{tag} gave {fields:?}, expected {wanted:?}"));
        }
    }
    Ok(())
}

#[test]
fn rejects_a_tag_that_is_not_a_release() -> Result<(), String> {
    // A missing field, a pre-release suffix, a leading space, a four-digit
    // field, and a bare version are all outside the documented shape.
    let cases = [
        "",
        "0.8.10",
        "libopenmpt-0.8",
        "libopenmpt-0.8.10.1",
        "libopenmpt-0.8.11rc1",
        "libopenmpt-0.8.1000",
        " libopenmpt-0.8.10",
        "libopenmpt-0.8.10 ",
        "libopenmpt-v0.8.10",
        "x",
    ];
    for tag in cases {
        let output = run(&["--check-tag", tag])?;
        if output.status.success() {
            return Err(format!("{tag:?} must be rejected"));
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("is not a release tag") && !stderr.contains("needs a tag") {
            return Err(format!(
                "{tag:?} failed with an unexpected message: {stderr}"
            ));
        }
        if !output.stdout.is_empty() {
            return Err(format!(
                "{tag:?} printed {:?} on stdout",
                String::from_utf8_lossy(&output.stdout)
            ));
        }
    }
    Ok(())
}

#[test]
fn check_tag_without_a_tag_names_the_fix() -> Result<(), String> {
    let output = run(&["--check-tag"])?;
    if output.status.success() {
        return Err("--check-tag with no tag must fail".to_owned());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.contains("--check-tag needs a tag") {
        return Err(format!(
            "the message is {stderr:?}, expected it to ask for a tag"
        ));
    }
    Ok(())
}
