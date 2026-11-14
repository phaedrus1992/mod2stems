//! The render phase of one export.
//!
//! The phase renders the unmuted reference first, because its trace is the flow
//! that every channel render must follow. It then renders one channel at a time
//! and stages each render in a float file, so the memory of the export does not
//! grow with the number of channels. Design: `docs/design.md` §4.1, §4.2.

use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use crate::encode::float_file;
use crate::export::Cancel;
use crate::export::ExportError;
use crate::export::ExportPlan;
use crate::export::Progress;
use crate::export::plan::load;
use crate::export::publish;
use crate::export::run::report;
use crate::export::stage::Stage;
use crate::stems::StemError;
use crate::stems::export_channel;
use crate::stems::reference_render;

/// The subdirectory of the staging directory that holds the scratch renders.
///
/// The scratch renders are working files. The export removes the subdirectory
/// before it publishes, because publication renames the whole staging
/// directory and a scratch file in it would travel with the export.
pub(crate) const SCRATCH_DIR: &str = "scratch";

/// The renders of one export, and the staged files that hold them.
///
/// The reference samples stay in memory, because every channel render compares
/// its trace with them. The channel renders go to staged float files, so only
/// one channel render occupies memory at a time.
#[derive(Debug)]
pub struct Renders {
    /// The interleaved stereo samples of the unmuted render.
    pub reference: Vec<f32>,
    /// The staged float files of the channel renders, in channel order.
    pub channels: Vec<PathBuf>,
    /// The longest frame count of every render, in stereo frames.
    pub frames: usize,
    /// Every warning that the library reported during the renders.
    pub warnings: Vec<String>,
}

/// Renders the reference and every channel of one export.
///
/// # Errors
///
/// Returns [`ExportError::Cancelled`] when the caller cancels the export,
/// [`ExportError::Stem`] when a render fails, [`ExportError::Encode`] when a
/// staged file cannot be written, and [`ExportError::Progress`] when the sink
/// cannot report.
pub fn render_all<W: Write>(
    plan: &ExportPlan,
    cancel: &Cancel,
    progress: &mut Progress<W>,
    staging: &Path,
) -> Result<Renders, ExportError> {
    check_cancel(cancel, staging)?;
    let scratch = staging.join(SCRATCH_DIR);
    std::fs::create_dir(&scratch).map_err(|error| ExportError::io(&scratch, &error))?;
    let mut module = load(plan.bytes())?;
    let mut warnings = module.take_log();
    let reference = reference_render(&mut module, plan.config(), &plan.policy())?;
    warnings.extend(module.take_log());
    report(progress, Stage::Reference)?;
    let reference_frames = frames_of(reference.samples())?;
    let mut longest = reference_frames;
    let mut channels = Vec::with_capacity(plan.channel_count());
    for channel in 0..plan.channels() {
        check_cancel(cancel, staging)?;
        let path = scratch.join(format!("stage-{channel:03}.wav"));
        let mut module = load(plan.bytes())?;
        let render = export_channel(
            &mut module,
            plan.config(),
            &plan.policy(),
            channel,
            &reference,
        )?;
        float_file::write(plan.rate(), &render.samples, &path)?;
        warnings.extend(module.take_log());
        longest = longest.max(frames_of(&render.samples)?);
        channels.push(path);
        report(progress, Stage::Channel(channel))?;
    }
    Ok(Renders {
        reference: reference.samples().to_vec(),
        channels,
        frames: longest,
        warnings,
    })
}

/// Returns the stereo frame count of one render.
///
/// A half frame cannot be placed on the timeline, so the call rejects one
/// rather than truncating it.
fn frames_of(samples: &[f32]) -> Result<usize, ExportError> {
    if samples.len().is_multiple_of(2) {
        return Ok(samples.len() / 2);
    }
    Err(ExportError::Stem(StemError::NotStereo {
        samples: samples.len(),
    }))
}

/// Fails when the caller cancelled the export.
///
/// The error carries the staging report, so the caller sees the work that the
/// cancelled export kept.
pub(crate) fn check_cancel(cancel: &Cancel, staging: &Path) -> Result<(), ExportError> {
    if !cancel.is_cancelled() {
        return Ok(());
    }
    let report = publish::report_or_path(staging);
    Err(ExportError::Cancelled(Box::new(report)))
}
