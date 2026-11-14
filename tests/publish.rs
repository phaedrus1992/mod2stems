//! Tests the staged publication of one export.
//!
//! The publication gate reads the manifest back from the staging directory. So
//! the set that publishes is the set that the manifest names, and nothing else.
//! A scratch file or a missing mix fails the gate before the rename.

use std::path::Path;
use std::path::PathBuf;

use mod2stems::export::ExportError;
use mod2stems::export::ExportRequest;
use mod2stems::export::publish;
use mod2stems::manifest::ExportDetails;
use mod2stems::manifest::MANIFEST_FILE;
use mod2stems::manifest::Manifest;
use mod2stems::manifest::README_FILE;
use mod2stems::manifest::StemEntry;

// The S3M builder reads its header helpers through `crate::header`, so every
// test binary mounts the helpers at its own root.
#[path = "support/fixture.rs"]
mod fixture;
#[path = "support/header.rs"]
mod header;

use fixture::export;
use fixture::manifest_of;
use fixture::require;
use fixture::require_eq;
use fixture::temporary;
use fixture::write_fixture;

/// Returns a manifest whose details name the given mixes.
fn manifest_with(mix: Option<&str>, reference: Option<&str>) -> Result<Manifest, String> {
    let details = ExportDetails {
        input_hash: "sha256:00".to_owned(),
        tool_version: "0.1.0".to_owned(),
        libopenmpt_version: "0.8.10+r25840".to_owned(),
        profile: "compatible".to_owned(),
        settings: Vec::new(),
        sample_rate: 48_000,
        layout: "stereo".to_owned(),
        frame_count: 4,
        end_policy: "song end".to_owned(),
        gain: 1.0,
        mix: mix.map(str::to_owned),
        reference_mix: reference.map(str::to_owned),
        format: "float WAV".to_owned(),
        dither: "none".to_owned(),
        quantization: "none (float)".to_owned(),
        subsong: 0,
        module_metadata: Vec::new(),
        warnings: Vec::new(),
        validation: "none".to_owned(),
    };
    let entry = StemEntry::channel(0, None, PathBuf::from("ch_00.wav"));
    Manifest::new(details, vec![entry]).map_err(|error| error.to_string())
}

/// Writes a staged set under `staging` and returns the staging path.
fn staged_set(name: &str, mix: Option<&str>, reference: Option<&str>) -> Result<PathBuf, String> {
    let workspace = temporary(name)?;
    let staging = workspace.join("stage");
    std::fs::create_dir_all(&staging).map_err(|error| error.to_string())?;
    let manifest = manifest_with(mix, reference)?;
    std::fs::write(staging.join("ch_00.wav"), b"audio").map_err(|error| error.to_string())?;
    std::fs::write(staging.join(README_FILE), b"readme").map_err(|error| error.to_string())?;
    manifest
        .write(&staging.join(MANIFEST_FILE))
        .map_err(|error| error.to_string())?;
    Ok(staging)
}

/// Returns the error text when publication refuses `staging`.
fn refusal(staging: &Path, destination: &Path) -> Result<String, String> {
    match publish::publish(staging, destination) {
        Ok(()) => Err("the set must not publish".to_owned()),
        Err(error) => Ok(error.to_string()),
    }
}

#[test]
fn the_published_set_holds_only_the_files_the_manifest_names() -> Result<(), String> {
    // The staging directory holds the scratch float renders of each channel, and
    // a rename of the whole directory would publish them. A DAW directory import
    // would then load each stem twice, and a FLAC export would carry float WAVs.
    let workspace = temporary("export-clean")?;
    let input = write_fixture(&workspace)?;
    let destination = workspace.join("stems");
    let mut request = ExportRequest::new(input, destination.clone());
    request.write_mix = true;
    export(&request)?;
    let manifest = manifest_of(&destination)?;
    let mut expected: Vec<String> = manifest
        .stems()
        .iter()
        .map(|entry| entry.path().display().to_string())
        .collect();
    if let Some(mix) = &manifest.details().mix {
        expected.push(mix.clone());
    }
    if let Some(reference) = &manifest.details().reference_mix {
        expected.push(reference.clone());
    }
    expected.push(MANIFEST_FILE.to_owned());
    expected.push(README_FILE.to_owned());
    let mut found = Vec::new();
    for entry in std::fs::read_dir(&destination).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        found.push(entry.file_name().to_string_lossy().into_owned());
    }
    found.sort();
    expected.sort();
    require_eq(
        &found,
        &expected,
        "the published set holds exactly the named files",
    )
}

#[test]
fn publish_refuses_a_set_that_holds_an_unnamed_file() -> Result<(), String> {
    // The negative check is the one that catches a scratch file: it is not
    // enough that every named file exists.
    let staging = staged_set("export-unnamed", None, None)?;
    std::fs::write(staging.join("stage-000.wav"), b"scratch").map_err(|error| error.to_string())?;
    let destination = staging
        .parent()
        .ok_or("the staging path has a parent")?
        .join("out");
    let message = refusal(&staging, &destination)?;
    require(
        message.contains("stage-000.wav"),
        &format!("the error names the unnamed file, got {message}"),
    )?;
    require(!destination.exists(), "a refused set publishes nothing")
}

#[test]
fn publish_refuses_a_set_that_lacks_a_named_mix() -> Result<(), String> {
    // The mix is named by the details and not by a stem entry, so a verification
    // pass that walks the stems alone never checks it.
    let staging = staged_set("export-nomix", Some("mix.wav"), None)?;
    let destination = staging
        .parent()
        .ok_or("the staging path has a parent")?
        .join("out");
    let message = refusal(&staging, &destination)?;
    require(
        message.contains("mix.wav"),
        &format!("the error names the missing mix, got {message}"),
    )?;
    require(!destination.exists(), "a refused set publishes nothing")
}

#[test]
fn publish_refuses_a_path_that_leaves_the_export() -> Result<(), String> {
    // A manifest is untrusted text. An absolute path would name a file outside
    // the export, and the gate would check that file instead of a staged one.
    let staging = staged_set("export-outside", None, None)?;
    let workspace = staging.parent().ok_or("the staging path has a parent")?;
    let outside = workspace.join("outside.wav");
    std::fs::write(&outside, b"audio").map_err(|error| error.to_string())?;
    let details = manifest_with(None, None)?.details().clone();
    let entry = StemEntry::channel(0, None, outside.clone());
    let manifest = Manifest::new(details, vec![entry]).map_err(|error| error.to_string())?;
    manifest
        .write(&staging.join(MANIFEST_FILE))
        .map_err(|error| error.to_string())?;
    let destination = workspace.join("out");
    let message = refusal(&staging, &destination)?;
    require(
        message.contains("leaves the export directory"),
        &format!("the error names the case, got {message}"),
    )?;
    require(!destination.exists(), "a refused set publishes nothing")
}

#[test]
fn a_report_that_cannot_read_its_directory_names_the_reason() -> Result<(), String> {
    // The failure report is the only record of the staging work. When the
    // listing fails, a report with no files reads as an empty directory, and
    // the caller then believes that the export kept nothing.
    let workspace = temporary("export-report-unreadable")?;
    let staging = workspace.join("absent");
    let report = publish::report_or_path(&staging);
    let text = report.to_string();
    require(
        text.contains(&staging.display().to_string()),
        &format!("the report names the path, got {text}"),
    )?;
    require(
        text.contains("Could not list"),
        &format!("the report names the failed listing, got {text}"),
    )?;
    require_eq(&report.retained().len(), &0, "the retained file count")
}

#[test]
fn create_staging_refuses_a_path_that_exists() -> Result<(), String> {
    // A check followed by a create lets another process take the path between
    // the two calls, so the create itself refuses an existing path.
    let workspace = temporary("export-staging-exists")?;
    let destination = workspace.join("stems");
    let first = publish::create_staging(&destination).map_err(|error| error.to_string())?;
    match publish::create_staging(&destination) {
        Ok(second) => Err(format!(
            "a second staging path must fail, got {}",
            second.display()
        )),
        Err(ExportError::Manifest(error)) => require(
            error.to_string().contains("staging directory"),
            &format!("the error names the staging path, got {error}"),
        ),
        Err(other) => Err(format!("the error must name the manifest, got {other}")),
    }?;
    require(first.is_dir(), "the first staging directory stays")
}
