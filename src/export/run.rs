//! The whole run of one export.
//!
//! The run resolves the plan, creates the staging directory, renders every
//! channel, writes the outputs, checks the manifest, and publishes. A failure
//! at any step leaves the staging directory in place and reports the files that
//! it holds. Design: `docs/design.md` §5.1, §5.2, §6.

use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use crate::export::Cancel;
use crate::export::ExportError;
use crate::export::ExportPlan;
use crate::export::ExportReport;
use crate::export::ExportRequest;
use crate::export::Progress;
use crate::export::amiga;
use crate::export::audio;
use crate::export::publish;
use crate::export::readme;
use crate::export::record::StagedFiles;
use crate::export::render::Renders;
use crate::export::render::SCRATCH_DIR;
use crate::export::render::check_cancel;
use crate::export::render::render_all;
use crate::export::stage::Stage;
use crate::manifest::README_FILE;
use crate::manifest::StemEntry;
use crate::stems::Timeline;

/// Runs one export, and returns what it published.
///
/// # Errors
///
/// Returns [`ExportError`] for every failed step. A failure never publishes a
/// partial set, and the error names the staging directory and its files.
pub fn run<W: Write>(
    request: &ExportRequest,
    cancel: &Cancel,
    progress: &mut Progress<W>,
) -> Result<ExportReport, ExportError> {
    let plan = ExportPlan::resolve(request)?;
    let files = usize::try_from(plan.channels()).unwrap_or(0);
    let stages = stage_count(&plan, request, files);
    report_begin(progress, stages, plan.render_count(), files)?;
    let staging = publish::create_staging(plan.destination())?;
    match execute(&plan, request, cancel, progress, &staging) {
        Ok(report) => Ok(report),
        Err(error) => Err(with_staging(error, &staging)),
    }
}

/// Runs every step that needs a staging directory.
fn execute<W: Write>(
    plan: &ExportPlan,
    request: &ExportRequest,
    cancel: &Cancel,
    progress: &mut Progress<W>,
    staging: &Path,
) -> Result<ExportReport, ExportError> {
    let mut renders = render_all(plan, cancel, progress, staging)?;
    amiga::check(plan, &renders.reference)?;
    let timeline = Timeline::from_frame_count(renders.frames)?;
    preflight(plan, request, &renders, timeline)?;
    let files = write_files(
        plan,
        request,
        &mut renders,
        timeline,
        progress,
        staging,
        cancel,
    )?;
    let details = plan.details(
        renders.frames,
        std::mem::take(&mut renders.warnings),
        &files,
    )?;
    check_cancel(cancel, staging)?;
    files.write_manifest(staging, details)?;
    write_readme(plan, renders.frames, staging)?;
    remove_scratch(staging)?;
    // The last check stands before the rename, so a cancelled export never
    // publishes a complete set that the caller no longer wants.
    check_cancel(cancel, staging)?;
    publish::publish(staging, plan.destination())?;
    report_published(progress)?;
    Ok(ExportReport {
        destination: plan.destination().to_path_buf(),
        renders: plan.render_count(),
        files: files.audio_paths().len(),
        frames: renders.frames,
    })
}

/// Checks every output against the integer range before any file is written.
///
/// The check measures only the files that the export writes. Each channel
/// render is read from its staged float file, so the check covers the whole set
/// without holding it in memory. The mix is measured as the sum that the export
/// writes, because a sum can exceed every stem that it holds.
fn preflight(
    plan: &ExportPlan,
    request: &ExportRequest,
    renders: &Renders,
    timeline: Timeline,
) -> Result<(), ExportError> {
    let mut peak = 0.0_f32;
    let mut sum: Vec<f32> = Vec::new();
    for path in &renders.channels {
        let samples = audio::layout_samples(path, timeline, plan.layout())?;
        peak = largest_peak(peak, &samples)?;
        if request.write_mix {
            add_into(&mut sum, &samples);
        }
    }
    if request.write_mix {
        peak = largest_peak(peak, &sum)?;
    }
    if request.write_reference_mix {
        let reference = audio::layout_buffer(&renders.reference, timeline, plan.layout())?;
        peak = largest_peak(peak, &reference)?;
    }
    if plan.gain_fits(peak, plan.encoding().gain()) {
        return Ok(());
    }
    let gain = plan.encoding().gain().value();
    Err(ExportError::OverRange {
        peak,
        format: plan.format().to_string(),
        highest: plan.format().peak(),
        gain,
        required: required_gain(plan.format().peak(), peak),
    })
}

/// Returns the largest common gain that brings `peak` inside `highest`.
///
/// The caller reaches this call only when `peak` is above the range, so `peak`
/// is finite and positive.
fn required_gain(highest: f32, peak: f32) -> f32 {
    if peak <= 0.0 {
        return highest;
    }
    highest / peak
}

/// Returns the larger of `current` and the peak of `samples`.
///
/// A sample that is not finite fails the check. `f32::max` ignores a NaN
/// operand, so the loop would otherwise read a broken render as a quiet one.
fn largest_peak(current: f32, samples: &[f32]) -> Result<f32, ExportError> {
    let mut peak = current;
    for (index, sample) in samples.iter().copied().enumerate() {
        if !sample.is_finite() {
            return Err(ExportError::from(crate::encode::EncodeError::NotFinite {
                index,
                value: sample,
            }));
        }
        peak = peak.max(sample.abs());
    }
    Ok(peak)
}

/// Writes every audio file of the export.
fn write_files<W: Write>(
    plan: &ExportPlan,
    request: &ExportRequest,
    renders: &mut Renders,
    timeline: Timeline,
    progress: &mut Progress<W>,
    staging: &Path,
    cancel: &Cancel,
) -> Result<StagedFiles, ExportError> {
    let mut files = StagedFiles::new();
    check_cancel(cancel, staging)?;
    if request.write_reference_mix {
        write_reference_mix(plan, renders, timeline, staging, &mut files)?;
        report(progress, Stage::ReferenceMix)?;
    }
    write_channel_stems(
        plan, renders, timeline, progress, staging, &mut files, cancel,
    )?;
    if request.write_mix {
        check_cancel(cancel, staging)?;
        write_mix(plan, renders, timeline, staging, &mut files)?;
        report(progress, Stage::Mix)?;
    }
    Ok(files)
}

/// Writes the unmuted render as audio, and releases its buffer.
fn write_reference_mix(
    plan: &ExportPlan,
    renders: &mut Renders,
    timeline: Timeline,
    staging: &Path,
    files: &mut StagedFiles,
) -> Result<(), ExportError> {
    let name = mix_name("reference", plan);
    let path = staging.join(&name);
    audio::write_buffer(
        &renders.reference,
        timeline,
        plan.layout(),
        plan.encoding(),
        &path,
    )?;
    renders.reference = Vec::new();
    files.add_reference_mix(path);
    Ok(())
}

/// Writes one file per tracker channel, silent channels included.
fn write_channel_stems<W: Write>(
    plan: &ExportPlan,
    renders: &Renders,
    timeline: Timeline,
    progress: &mut Progress<W>,
    staging: &Path,
    files: &mut StagedFiles,
    cancel: &Cancel,
) -> Result<(), ExportError> {
    for (index, staged) in renders.channels.iter().enumerate() {
        check_cancel(cancel, staging)?;
        let channel = channel_of(index);
        let name = plan.stem_name(channel);
        let path = staging.join(&name);
        audio::write_output(staged, timeline, plan.layout(), plan.encoding(), &path)?;
        files.add_stem(stem_entry(plan, channel, name), path);
        report(progress, Stage::File(channel))?;
    }
    Ok(())
}

/// Writes the sum of the channel stems.
///
/// The sum runs on the float stereo buffers, so the mix holds the same audio as
/// the stems at the same gain, and the one downmix runs on the sum. A downmix is
/// linear, so the mean of a sum equals the sum of the means.
fn write_mix(
    plan: &ExportPlan,
    renders: &Renders,
    timeline: Timeline,
    staging: &Path,
    files: &mut StagedFiles,
) -> Result<(), ExportError> {
    let mut sum: Vec<f32> = Vec::new();
    for path in &renders.channels {
        let samples = audio::padded_samples(path, timeline)?;
        add_into(&mut sum, &samples);
    }
    let name = mix_name("mix", plan);
    let path = staging.join(&name);
    audio::write_buffer(&sum, timeline, plan.layout(), plan.encoding(), &path)?;
    files.add_mix(path);
    Ok(())
}

/// Adds `samples` into `sum`, and seeds `sum` from the first buffer.
fn add_into(sum: &mut Vec<f32>, samples: &[f32]) {
    if sum.is_empty() {
        *sum = samples.to_vec();
        return;
    }
    for (total, sample) in sum.iter_mut().zip(samples) {
        *total += sample;
    }
}

/// Returns the stem entry of one channel.
fn stem_entry(plan: &ExportPlan, channel: i32, name: String) -> StemEntry {
    StemEntry::channel(channel, plan.channel_name(channel), PathBuf::from(name))
}

/// Returns the channel index of a position in the render list.
fn channel_of(index: usize) -> i32 {
    i32::try_from(index).unwrap_or(i32::MAX)
}

/// Removes the scratch directory of the render phase.
///
/// The scratch directory is not part of the export. A failure to remove it does
/// not stop the run, because `publish` refuses a set that holds an unnamed file
/// and reports it.
fn remove_scratch(staging: &Path) -> Result<(), ExportError> {
    let scratch = staging.join(SCRATCH_DIR);
    match std::fs::remove_dir_all(&scratch) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(ExportError::io(&scratch, &error)),
    }
}

/// Writes the README that labels the export.
fn write_readme(plan: &ExportPlan, frames: usize, staging: &Path) -> Result<(), ExportError> {
    let path = staging.join(README_FILE);
    let text = readme::text(plan, frames);
    std::fs::write(&path, text).map_err(|error| ExportError::io(&path, &error))
}

/// Returns the file name of one mix.
fn mix_name(base: &str, plan: &ExportPlan) -> String {
    format!("{base}.{}", plan.format().extension())
}

/// Attaches the staging report to a failure.
///
/// A cancelled export already carries its report, so the call passes it
/// through untouched.
fn with_staging(error: ExportError, staging: &Path) -> ExportError {
    if error.staging().is_some() {
        return error;
    }
    ExportError::Failed {
        cause: Box::new(error),
        staging: Box::new(publish::report_or_path(staging)),
    }
}

/// Returns the number of progress lines that the run prints.
///
/// The reference and each channel report one render line, each channel file
/// reports one line, each optional mix reports one line, and the rename reports
/// one line. The `amiga` check renders the module twice more and reports no
/// line, so its renders are not counted here.
fn stage_count(plan: &ExportPlan, request: &ExportRequest, files: usize) -> usize {
    let renders = usize::try_from(plan.channels())
        .unwrap_or(0)
        .saturating_add(1);
    let mixes = usize::from(request.write_mix) + usize::from(request.write_reference_mix);
    renders
        .saturating_add(files)
        .saturating_add(mixes)
        .saturating_add(1)
}

/// Reports the size of the run, and converts a sink failure into an export
/// error.
fn report_begin<W: Write>(
    progress: &mut Progress<W>,
    stages: usize,
    renders: usize,
    files: usize,
) -> Result<(), ExportError> {
    progress
        .begin(stages, renders, files)
        .map_err(|error| ExportError::progress(&error, false))
}

/// Reports one stage that runs before the rename, and converts a sink failure
/// into an export error.
fn report<W: Write>(progress: &mut Progress<W>, stage: Stage) -> Result<(), ExportError> {
    progress
        .stage(stage)
        .map_err(|error| ExportError::progress(&error, false))
}

/// Reports the publish stage, and converts a sink failure into an export error.
///
/// The rename already happened, so the failure must not claim that nothing
/// published.
fn report_published<W: Write>(progress: &mut Progress<W>) -> Result<(), ExportError> {
    progress
        .stage(Stage::Publish)
        .map_err(|error| ExportError::progress(&error, true))
}
