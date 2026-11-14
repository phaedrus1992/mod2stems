//! The output layout of one render, and the file that the export writes.
//!
//! A render arrives as interleaved stereo at the render rate. The export pads
//! it to the common timeline, downmixes it to the output layout, applies the
//! common gain, and encodes. The conversion happens on float samples only, so
//! no intermediate audio passes the integer path. Design: `docs/design.md`
//! §4.2, §5.1.

use std::path::Path;

use crate::encode::Encoding;
use crate::encode::encode;
use crate::encode::float_file;
use crate::export::ExportError;
use crate::stems::ChannelLayout;
use crate::stems::Timeline;
use crate::stems::downmix;

/// Returns one stereo render in the output layout.
///
/// The call pads the render to the common timeline first, because the timeline
/// counts stereo frames and a mono buffer counts twice as many. It downmixes
/// after the padding.
///
/// # Errors
///
/// Returns [`ExportError::Stem`] when the render is past the timeline or holds
/// a half frame.
pub fn layout_buffer(
    samples: &[f32],
    timeline: Timeline,
    layout: ChannelLayout,
) -> Result<Vec<f32>, ExportError> {
    let padded = timeline.pad(samples)?;
    downmix(layout, &padded).map_err(ExportError::from)
}

/// Returns one stereo render in the output layout, and releases the input.
///
/// Returns the staged stereo render at `path`, padded to the common timeline.
///
/// The call keeps the stereo layout. Summing padded stereo buffers is exact for
/// either output layout: a downmix is linear, so the downmix of a sum equals the
/// sum of the downmixes. A mono export that summed downmixed buffers instead
/// would downmix them a second time, because the sum has one sample per frame
/// and the downmix expects two.
///
/// # Errors
///
/// Returns [`ExportError::Encode`] when the staged file cannot be read, and
/// [`ExportError::Stem`] when the render is past the timeline.
pub fn padded_samples(path: &Path, timeline: Timeline) -> Result<Vec<f32>, ExportError> {
    let audio = float_file::read(path)?;
    timeline.pad(&audio.samples).map_err(ExportError::from)
}

/// Returns the staged stereo render at `path`, in the output layout.
///
/// # Errors
///
/// Returns [`ExportError::Encode`] when the staged file cannot be read, and
/// [`ExportError::Stem`] when the render is past the timeline.
pub fn layout_samples(
    path: &Path,
    timeline: Timeline,
    layout: ChannelLayout,
) -> Result<Vec<f32>, ExportError> {
    let audio = float_file::read(path)?;
    layout_buffer(&audio.samples, timeline, layout)
}

/// Writes one staged stereo render to `output` under `encoding`.
///
/// # Errors
///
/// Returns [`ExportError::Encode`] when the staged file cannot be read or the
/// output cannot be written, and [`ExportError::Stem`] when the render is past
/// the timeline.
pub fn write_output(
    path: &Path,
    timeline: Timeline,
    layout: ChannelLayout,
    encoding: Encoding,
    output: &Path,
) -> Result<(), ExportError> {
    let samples = layout_samples(path, timeline, layout)?;
    encode(&encoding, &samples, output).map_err(ExportError::from)
}

/// Writes one stereo render to `output` under `encoding`.
///
/// # Errors
///
/// Returns [`ExportError::Stem`] when the render is past the timeline, and
/// [`ExportError::Encode`] when the output cannot be written.
pub fn write_buffer(
    samples: &[f32],
    timeline: Timeline,
    layout: ChannelLayout,
    encoding: Encoding,
    output: &Path,
) -> Result<(), ExportError> {
    let converted = layout_buffer(samples, timeline, layout)?;
    encode(&encoding, &converted, output).map_err(ExportError::from)
}
