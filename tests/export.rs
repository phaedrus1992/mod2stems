//! Tests the staged export run.
//!
//! The checks cover the published set, the manifest that names it, the mono
//! layout, the collision rule, and cancellation. The mix checks live in
//! `tests/mix.rs`. The synthetic module plays two short orders, so a whole
//! export runs in a test.

use mod2stems::encode::Format;
use mod2stems::export::Cancel;
use mod2stems::export::ExportError;
use mod2stems::export::ExportRequest;
use mod2stems::export::Progress;
use mod2stems::export::run;
use mod2stems::manifest::MANIFEST_FILE;
use mod2stems::manifest::README_FILE;
use mod2stems::stems::ChannelLayout;

#[path = "support/fixture.rs"]
mod fixture;
#[path = "support/header.rs"]
mod header;
use fixture::export;
use fixture::manifest_of;
use fixture::require;
use fixture::require_eq;
use fixture::require_val;
use fixture::temporary;
use fixture::write_fixture;

#[test]
fn an_export_publishes_one_file_per_channel_and_a_manifest() -> Result<(), String> {
    let workspace = temporary("export-full")?;
    let input = write_fixture(&workspace)?;
    let destination = workspace.join("stems");
    let request = ExportRequest::new(input, destination.clone());
    let report = export(&request)?;
    require(report.files >= 1, "the export writes at least one file")?;
    require(
        destination.join(MANIFEST_FILE).is_file(),
        "the manifest exists",
    )?;
    require(destination.join(README_FILE).is_file(), "the README exists")?;
    let manifest = manifest_of(&destination)?;
    require_val(
        manifest.stems().len(),
        report.files,
        "the manifest names the set",
    )?;
    for entry in manifest.stems() {
        let path = destination.join(entry.path());
        require(
            path.is_file(),
            &format!("the manifest names {}, which exists", path.display()),
        )?;
    }
    Ok(())
}

#[test]
fn the_manifest_records_the_resolved_settings() -> Result<(), String> {
    let workspace = temporary("export-details")?;
    let input = write_fixture(&workspace)?;
    let destination = workspace.join("stems");
    let request = ExportRequest::new(input, destination.clone());
    export(&request)?;
    let manifest = manifest_of(&destination)?;
    let details = manifest.details();
    require(
        details.input_hash.starts_with("sha256:"),
        "the input is named by digest",
    )?;
    require_val(details.sample_rate, 48_000, "the sample rate")?;
    require(
        details
            .settings
            .iter()
            .any(|(name, _)| name == "play.at_end"),
        "the manifest records the playback settings",
    )?;
    require(
        !details.libopenmpt_version.is_empty(),
        "the manifest records the library version",
    )?;
    require_val(details.gain, 1.0, "the common gain")?;
    Ok(())
}

#[test]
fn the_two_mixes_are_recorded_and_written() -> Result<(), String> {
    let workspace = temporary("export-mixes")?;
    let input = write_fixture(&workspace)?;
    let destination = workspace.join("stems");
    let mut request = ExportRequest::new(input, destination.clone());
    request.write_mix = true;
    request.write_reference_mix = true;
    export(&request)?;
    let manifest = manifest_of(&destination)?;
    let details = manifest.details();
    let mix = details.mix.clone().ok_or("the manifest names no mix")?;
    let reference = details
        .reference_mix
        .clone()
        .ok_or("the manifest names no reference mix")?;
    require(destination.join(&mix).is_file(), "the mix exists")?;
    require(
        destination.join(&reference).is_file(),
        "the reference mix exists",
    )?;
    Ok(())
}

#[test]
fn a_mono_export_writes_mono_files() -> Result<(), String> {
    let workspace = temporary("export-mono")?;
    let input = write_fixture(&workspace)?;
    let destination = workspace.join("stems");
    let mut request = ExportRequest::new(input, destination.clone());
    request.layout = ChannelLayout::Mono;
    export(&request)?;
    let manifest = manifest_of(&destination)?;
    require_eq(&manifest.details().layout, &"mono".to_owned(), "the layout")?;
    for entry in manifest.stems() {
        let path = destination.join(entry.path());
        let reader = hound::WavReader::open(&path).map_err(|e| e.to_string())?;
        require_val(
            reader.spec().channels,
            1,
            &format!("{} is mono", path.display()),
        )?;
    }
    Ok(())
}

#[test]
fn an_integer_export_records_the_quantization() -> Result<(), String> {
    let workspace = temporary("export-flac")?;
    let input = write_fixture(&workspace)?;
    let destination = workspace.join("stems");
    let mut request = ExportRequest::new(input, destination.clone());
    request.format = Format::FlacI16;
    match export(&request) {
        Ok(_) => {
            let manifest = manifest_of(&destination)?;
            require(
                manifest.details().quantization.contains("integer"),
                "the manifest records the quantization",
            )?;
        }
        Err(message) => {
            // The fixture can be louder than 16-bit full scale. The run must
            // then fail before it publishes anything, and name the fix.
            require(
                message.contains("--gain"),
                &format!("the error names the fix, got {message}"),
            )?;
            require(!destination.exists(), "a failed export publishes nothing")?;
        }
    }
    Ok(())
}

#[test]
fn a_missing_input_names_the_read_failure() -> Result<(), String> {
    let workspace = temporary("export-missing-input")?;
    let input = workspace.join("absent.s3m");
    let destination = workspace.join("stems");
    let request = ExportRequest::new(input.clone(), destination);
    match export(&request) {
        Ok(_) => Err("a missing input must fail".to_owned()),
        Err(message) => {
            require(
                message.contains(&input.display().to_string()),
                &format!("the error names the input, got {message}"),
            )?;
            require(
                message.contains("Cannot read the input module"),
                &format!("the error names the read, got {message}"),
            )
        }
    }
}

#[test]
fn an_over_range_export_states_a_gain_that_fits() -> Result<(), String> {
    // The required gain is the largest factor that still fits the format. The
    // old message multiplied the current gain by that factor, so a caller who
    // applied it landed in the same failure.
    let workspace = temporary("export-over-range")?;
    let input = write_fixture(&workspace)?;
    let destination = workspace.join("stems");
    let mut request = ExportRequest::new(input, destination.clone());
    request.format = Format::FlacI16;
    request.gain = 8.0;
    let Err(message) = export(&request) else {
        return Err("a gain that overflows 16-bit must fail".to_owned());
    };
    require(!destination.exists(), "a failed export publishes nothing")?;
    let marker = "Set --gain to ";
    let start = message
        .find(marker)
        .ok_or_else(|| format!("the error states no gain, got {message}"))?
        + marker.len();
    let rest = message.get(start..).unwrap_or_default();
    let end = rest.find(' ').unwrap_or(rest.len());
    let required: f32 = rest
        .get(..end)
        .ok_or("the error states no gain")?
        .parse()
        .map_err(|_| format!("the error states no gain, got {message}"))?;
    require(
        required < 8.0_f32,
        &format!("the stated gain of {required} must fit the format"),
    )?;
    Ok(())
}

#[test]
fn an_existing_destination_is_refused() -> Result<(), String> {
    let workspace = temporary("export-exists")?;
    let input = write_fixture(&workspace)?;
    let destination = workspace.join("stems");
    std::fs::create_dir_all(&destination).map_err(|e| e.to_string())?;
    let request = ExportRequest::new(input, destination);
    match export(&request) {
        Ok(_) => Err("an existing destination must fail".to_owned()),
        Err(message) => require(
            message.contains("already exists"),
            &format!("the error names the case, got {message}"),
        ),
    }
}

#[test]
fn a_cancelled_export_publishes_nothing() -> Result<(), String> {
    let workspace = temporary("export-cancel")?;
    let input = write_fixture(&workspace)?;
    let destination = workspace.join("stems");
    let request = ExportRequest::new(input, destination.clone());
    let cancel = Cancel::new();
    cancel.cancel();
    let mut progress = Progress::new(Vec::new());
    let Err(error) = run(&request, &cancel, &mut progress) else {
        return Err("a cancelled export must fail".to_owned());
    };
    require(
        matches!(error, ExportError::Cancelled(_)),
        "the error names the cancellation",
    )?;
    require(
        !destination.exists(),
        "a cancelled export publishes nothing",
    )?;
    let staging = error
        .staging()
        .ok_or("the error names the staging directory")?;
    require(
        staging.path().exists(),
        "the staging directory stays for inspection",
    )
}

/// The sink that cancels the export when it writes the line that names a
/// `needle`.
///
/// The cancellation lands between two stages, which is where a person presses
/// the interrupt key: a progress line arrives, the person reacts, and the
/// export continues.
struct CancelOn {
    needle: &'static str,
    cancel: Cancel,
}

impl std::io::Write for CancelOn {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        let text = String::from_utf8_lossy(buffer);
        if text.contains(self.needle) {
            self.cancel.cancel();
        }
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn a_cancel_after_the_last_render_publishes_nothing() -> Result<(), String> {
    // The renders are the long step, so a person cancels during them. Every
    // later stage must still reach the flag, and the rename must not run.
    let workspace = temporary("export-cancel-late")?;
    let input = write_fixture(&workspace)?;
    let destination = workspace.join("stems");
    let request = ExportRequest::new(input, destination.clone());
    let cancel = Cancel::new();
    let mut progress = Progress::new(CancelOn {
        needle: "rendered channel 1",
        cancel: cancel.clone(),
    });
    let Err(error) = run(&request, &cancel, &mut progress) else {
        return Err("a cancelled export must fail".to_owned());
    };
    require(
        matches!(error, ExportError::Cancelled(_)),
        &format!("the error names the cancellation, got {error}"),
    )?;
    require(
        !destination.exists(),
        "a cancelled export publishes nothing",
    )
}

/// The sink that fails on the line that names a `needle`.
///
/// The publish line is the last one, so the sink must accept every earlier
/// line and fail only on the one that proves the export reached the rename.
struct FailOn {
    needle: &'static str,
}

impl std::io::Write for FailOn {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        let text = String::from_utf8_lossy(buffer);
        if text.contains(self.needle) {
            return Err(std::io::Error::other("the sink stopped"));
        }
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn a_sink_failure_after_the_rename_says_the_export_published() -> Result<(), String> {
    // The rename is the last step, so the export is complete. A message that
    // says nothing published sends the caller hunting for a partial directory
    // that does not exist.
    let workspace = temporary("export-publish-sink")?;
    let input = write_fixture(&workspace)?;
    let destination = workspace.join("stems");
    let request = ExportRequest::new(input, destination.clone());
    let cancel = Cancel::new();
    let mut progress = Progress::new(FailOn {
        needle: "published the export",
    });
    let Err(error) = run(&request, &cancel, &mut progress) else {
        return Err("a failed publish report must fail the run".to_owned());
    };
    let message = error.to_string();
    require(
        message.contains("The export published"),
        &format!("the error says the export published, got {message}"),
    )?;
    require(
        destination.join(MANIFEST_FILE).is_file(),
        "the export publishes before the sink fails",
    )
}

#[test]
fn pair_stems_are_refused_with_the_issue_numbers() -> Result<(), String> {
    let workspace = temporary("export-pairs")?;
    let input = write_fixture(&workspace)?;
    let destination = workspace.join("stems");
    let mut request = ExportRequest::new(input, destination.clone());
    request.instrument_channel_stems = true;
    match export(&request) {
        Ok(_) => Err("pair stems must fail in this release".to_owned()),
        Err(message) => {
            require(
                message.contains("#12"),
                &format!("the error names the follow-up issues, got {message}"),
            )?;
            require(!destination.exists(), "a failed export publishes nothing")
        }
    }
}

#[test]
fn a_resolved_rate_reaches_the_manifest() -> Result<(), String> {
    let workspace = temporary("export-rate")?;
    let input = write_fixture(&workspace)?;
    let destination = workspace.join("stems");
    let mut request = ExportRequest::new(input, destination.clone());
    request.rate = Some(22_050);
    export(&request)?;
    let manifest = manifest_of(&destination)?;
    require_val(manifest.details().sample_rate, 22_050, "the sample rate")
}
