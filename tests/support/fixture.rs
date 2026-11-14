//! The shared fixture and assertions of the export tests.
//!
//! Two test binaries mount this file, so each one compiles its own copy. The
//! synthetic module plays two short orders, so a whole export runs in a test.

#![allow(dead_code, reason = "a test binary mounts only the helpers it uses")]

use std::fmt::Debug;
use std::path::Path;
use std::path::PathBuf;

use mod2stems::export::Cancel;
use mod2stems::export::ExportReport;
use mod2stems::export::ExportRequest;
use mod2stems::export::Progress;
use mod2stems::export::run;
use mod2stems::manifest::MANIFEST_FILE;
use mod2stems::manifest::Manifest;

#[path = "s3m.rs"]
mod s3m;

/// Returns a new empty directory under the test temporary directory.
pub fn temporary(name: &str) -> Result<PathBuf, String> {
    let base = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    if base.exists() {
        std::fs::remove_dir_all(&base)
            .map_err(|error| format!("cannot clear {}: {error}", base.display()))?;
    }
    std::fs::create_dir_all(&base)
        .map_err(|error| format!("cannot create {}: {error}", base.display()))?;
    Ok(base)
}

/// Writes the synthetic S3M fixture into `directory` and returns its path.
///
/// The vendored corpus holds a minute-long song, and a whole export of it takes
/// a minute. The synthetic module plays two short orders, so a test covers the
/// whole run in a second or two.
pub fn write_fixture(directory: &Path) -> Result<PathBuf, String> {
    let path = directory.join("fixture.s3m");
    std::fs::write(&path, s3m::muted_flow_s3m())
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    Ok(path)
}

/// Returns an error that names `what` when `condition` is false.
pub fn require(condition: bool, what: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(what.to_owned())
    }
}

/// Returns an error that names both values when they differ.
///
/// The values are borrowed, so a caller passes a large value without a clone.
pub fn require_eq<T: Debug + PartialEq>(
    actual: &T,
    expected: &T,
    what: &str,
) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{what}: got {actual:?}, expected {expected:?}"))
    }
}

/// Returns an error that names both values when two small values differ.
pub fn require_val<T: Copy + Debug + PartialEq>(
    actual: T,
    expected: T,
    what: &str,
) -> Result<(), String> {
    require_eq(&actual, &expected, what)
}

/// Runs one export, and returns the report or the error text.
pub fn export(request: &ExportRequest) -> Result<ExportReport, String> {
    let cancel = Cancel::new();
    let mut progress = Progress::new(Vec::new());
    run(request, &cancel, &mut progress).map_err(|error| error.to_string())
}

/// Reads the manifest of one published export.
pub fn manifest_of(destination: &Path) -> Result<Manifest, String> {
    Manifest::read(&destination.join(MANIFEST_FILE)).map_err(|error| error.to_string())
}
