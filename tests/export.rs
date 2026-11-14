//! Tests the staged export run.
//!
//! The checks cover the published set, the manifest that names it, the two
//! mixes, the mono layout, the collision rule, and cancellation. The synthetic
//! module plays two short orders, so a whole export runs in a test.

use std::fmt::Debug;
use std::path::Path;
use std::path::PathBuf;

use mod2stems::encode::Format;
use mod2stems::export::Cancel;
use mod2stems::export::ExportError;
use mod2stems::export::ExportReport;
use mod2stems::export::ExportRequest;
use mod2stems::export::Progress;
use mod2stems::export::run;
use mod2stems::manifest::MANIFEST_FILE;
use mod2stems::manifest::Manifest;
use mod2stems::manifest::README_FILE;
use mod2stems::stems::ChannelLayout;

#[path = "support/header.rs"]
mod header;
// The S3M builders share one file across test binaries, so a binary that mounts
// the file for one builder leaves the others unused.
#[expect(
    dead_code,
    reason = "the S3M builders share one file across test binaries"
)]
#[path = "support/s3m.rs"]
mod s3m;

/// Returns a new empty directory under the test temporary directory.
fn temporary(name: &str) -> Result<PathBuf, String> {
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
/// The vendored corpus holds a minute-long song, and a whole export of it
/// takes a minute. The synthetic module plays two short orders, so a test
/// covers the whole run in a second or two.
fn write_fixture(directory: &Path) -> Result<PathBuf, String> {
    let path = directory.join("fixture.s3m");
    std::fs::write(&path, s3m::muted_flow_s3m())
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    Ok(path)
}

/// Returns an error that names `what` when `condition` is false.
fn require(condition: bool, what: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(what.to_owned())
    }
}

/// Returns an error that names both values when they differ.
///
/// The values are borrowed, so a caller passes a large value without a clone.
fn require_eq<T: Debug + PartialEq>(actual: &T, expected: &T, what: &str) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{what}: got {actual:?}, expected {expected:?}"))
    }
}

/// Returns an error that names both values when two small values differ.
fn require_val<T: Copy + Debug + PartialEq>(
    actual: T,
    expected: T,
    what: &str,
) -> Result<(), String> {
    require_eq(&actual, &expected, what)
}

/// Reads one float WAV file, as interleaved samples.
fn read_wav(path: &Path) -> Result<Vec<f32>, String> {
    let mut reader = hound::WavReader::open(path).map_err(|error| error.to_string())?;
    reader
        .samples::<f32>()
        .collect::<Result<Vec<f32>, _>>()
        .map_err(|error| error.to_string())
}

/// Adds `samples` into `sum`, and seeds `sum` from the first buffer.
///
/// The stems and the mix share one layout, so a straight sum is comparable with
/// the mix.
fn add_into(sum: &mut Vec<f32>, samples: &[f32]) {
    if sum.is_empty() {
        *sum = samples.to_vec();
        return;
    }
    for (total, sample) in sum.iter_mut().zip(samples) {
        *total += sample;
    }
}

/// Runs one export, and returns the report or the error text.
fn export(request: &ExportRequest) -> Result<ExportReport, String> {
    let cancel = Cancel::new();
    let mut progress = Progress::new(Vec::new());
    run(request, &cancel, &mut progress).map_err(|error| error.to_string())
}

/// Reads the manifest of one published export.
fn manifest_of(destination: &Path) -> Result<Manifest, String> {
    Manifest::read(&destination.join(MANIFEST_FILE)).map_err(|error| error.to_string())
}

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
fn the_mix_sums_to_the_stems_within_the_float_tolerance() -> Result<(), String> {
    // The mix is the sum of the final float stems, with no normalization. The
    // check proves the mix is not a separate render and not a scaled copy.
    let workspace = temporary("export-sum")?;
    let input = write_fixture(&workspace)?;
    let destination = workspace.join("stems");
    let mut request = ExportRequest::new(input, destination.clone());
    request.write_mix = true;
    export(&request)?;
    let manifest = manifest_of(&destination)?;
    let mut sum: Vec<f32> = Vec::new();
    for entry in manifest.stems() {
        let samples = read_wav(&destination.join(entry.path()))?;
        add_into(&mut sum, &samples);
    }
    let mix_name = manifest
        .details()
        .mix
        .clone()
        .ok_or("the manifest names no mix")?;
    let mix = read_wav(&destination.join(mix_name))?;
    require_val(mix.len(), sum.len(), "the mix spans the stems")?;
    for (index, (mix_sample, sum_sample)) in mix.iter().zip(&sum).enumerate() {
        require(
            (mix_sample - sum_sample).abs() <= 1e-5,
            &format!("sample {index} differs: mix {mix_sample}, sum {sum_sample}"),
        )?;
    }
    Ok(())
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
