//! Tests the version line that the tool prints.

use std::process::Command;
use std::process::Output;

/// Runs the binary and returns its output.
fn run() -> Result<Output, String> {
    Command::new(env!("CARGO_BIN_EXE_mod2stems"))
        .output()
        .map_err(|error| format!("cannot run the tool: {error}"))
}

#[test]
fn prints_versions() -> Result<(), String> {
    let output = run()?;
    if !output.status.success() {
        return Err(format!("the tool exited with {}", output.status));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let wanted = [
        format!("mod2stems {}", env!("CARGO_PKG_VERSION")),
        "libopenmpt 0.8.10".to_owned(),
        "r25840".to_owned(),
    ];
    for text in &wanted {
        if !stdout.contains(text) {
            return Err(format!("the output {stdout:?} holds no {text:?}"));
        }
    }
    Ok(())
}

#[test]
fn prints_one_line() -> Result<(), String> {
    let output = run()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.lines().count() != 1 {
        return Err(format!("the output {stdout:?} has more than one line"));
    }
    Ok(())
}
