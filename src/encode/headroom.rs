//! The level preflight that an integer export runs before it encodes.
//!
//! The integer path cannot hold a sample outside its range, and it never clips
//! silently. The export therefore measures every requested output first, and
//! either fails with the peak in front of the caller or applies one common gain
//! that the caller chose. Design: `docs/design.md` §5.1.

use crate::encode::EncodeError;
use crate::encode::Format;

/// The measured peak across the outputs of one export.
///
/// The caller reads [`Self::fits`] to decide whether the integer path can run,
/// and [`Self::outputs_over`] to see how much of the export the range rejects.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Headroom {
    format: Format,
    peak: f32,
    outputs: usize,
    outputs_over: usize,
}

impl Headroom {
    /// Returns the format that the measurement checked against.
    #[must_use]
    pub fn format(self) -> Format {
        self.format
    }

    /// Returns the largest sample magnitude that the export holds.
    #[must_use]
    pub fn peak(self) -> f32 {
        self.peak
    }

    /// Returns the number of outputs that the measurement covered.
    #[must_use]
    pub fn outputs(self) -> usize {
        self.outputs
    }

    /// Returns the number of outputs with a sample past the format range.
    #[must_use]
    pub fn outputs_over(self) -> usize {
        self.outputs_over
    }

    /// Returns true when every output fits the format range.
    #[must_use]
    pub fn fits(self) -> bool {
        self.outputs_over == 0
    }

    /// Returns the gain that brings the peak inside the format range.
    ///
    /// The value is the format peak divided by the measured peak, so the
    /// loudest sample lands exactly on full scale. A value of one means the
    /// export already fits. The caller applies the same value to every output,
    /// which keeps the relative levels of the set.
    ///
    /// A gain below one is an attenuation. This call never returns a boost,
    /// because the preflight exists to fit the range, not to raise the level
    /// (`docs/design.md` §5.1).
    #[must_use]
    pub fn required_gain(self) -> f32 {
        let target = self.format.peak();
        if self.peak <= target || !self.peak.is_finite() {
            return 1.0;
        }
        target / self.peak
    }

    /// Returns the measurement that covers this output and another one.
    ///
    /// An export renders its outputs one at a time and stages each one in a
    /// file, so it measures each render as it arrives and merges the results.
    /// The peak and the counts then cover every output without holding them
    /// all in memory.
    ///
    /// # Errors
    ///
    /// Returns [`EncodeError::FormatMismatch`] when the two measurements checked
    /// against different formats, because their peaks are not comparable.
    pub fn merge(self, other: Self) -> Result<Self, EncodeError> {
        if self.format != other.format {
            return Err(EncodeError::FormatMismatch);
        }
        Ok(Self {
            format: self.format,
            peak: self.peak.max(other.peak),
            outputs: self.outputs.saturating_add(other.outputs),
            outputs_over: self.outputs_over.saturating_add(other.outputs_over),
        })
    }

    /// Returns the error that an over-range export reports.
    ///
    /// # Errors
    ///
    /// Returns [`EncodeError::OverRange`] when at least one output is past the
    /// range of the format.
    pub fn require_fit(self) -> Result<(), EncodeError> {
        if self.fits() {
            return Ok(());
        }
        Err(EncodeError::OverRange {
            index: 0,
            value: self.peak,
            format: self.format_name(),
            highest: self.format.peak(),
        })
    }

    /// Returns the format name that an error message reports.
    fn format_name(self) -> &'static str {
        match self.format {
            Format::WavF32 => "float WAV",
            Format::FlacI16 => "16-bit FLAC",
            Format::FlacI24 => "24-bit FLAC",
        }
    }
}

/// Measures the peak of every output against the range of `format`.
///
/// The caller passes the final float buffer of each output that the export
/// will write. A buffer of an odd sample count fails, because the measurement
/// cannot name a frame that holds one sample.
///
/// # Errors
///
/// Returns [`EncodeError::EmptyBuffer`] when no output is given, and
/// [`EncodeError::BadFrame`] when one holds no whole frame. Returns
/// [`EncodeError::NotFinite`] for a sample that is not finite.
pub fn preflight_headroom<'a, I>(format: Format, outputs: I) -> Result<Headroom, EncodeError>
where
    I: IntoIterator<Item = &'a [f32]>,
{
    preflight_headroom_for(format, 2, outputs)
}

/// Measures the peak of every output of a named channel layout.
///
/// The layout decides the frame shape, so a mono export passes its own count
/// instead of the stereo default of [`preflight_headroom`].
///
/// # Errors
///
/// Returns the same errors as [`preflight_headroom`].
pub fn preflight_headroom_for<'a, I>(
    format: Format,
    channels: usize,
    outputs: I,
) -> Result<Headroom, EncodeError>
where
    I: IntoIterator<Item = &'a [f32]>,
{
    let mut count = 0_usize;
    let mut over = 0_usize;
    let mut peak = 0.0_f32;
    let target = format.peak();
    for samples in outputs {
        check_shape(samples, channels)?;
        count = count.saturating_add(1);
        let mut output_over = false;
        for (index, sample) in samples.iter().copied().enumerate() {
            if !sample.is_finite() {
                return Err(EncodeError::NotFinite {
                    index,
                    value: sample,
                });
            }
            let magnitude = sample.abs();
            peak = peak.max(magnitude);
            if magnitude > target {
                output_over = true;
            }
        }
        if output_over {
            over = over.saturating_add(1);
        }
    }
    if count == 0 {
        return Err(EncodeError::EmptyBuffer);
    }
    Ok(Headroom {
        format,
        peak,
        outputs: count,
        outputs_over: over,
    })
}

/// Returns the frame count of an interleaved buffer.
///
/// # Errors
///
/// Returns [`EncodeError::EmptyBuffer`] when the buffer holds no sample, and
/// [`EncodeError::BadFrame`] when the sample count is not a whole number of
/// frames of `channels`, or when `channels` is zero.
pub(crate) fn check_shape(samples: &[f32], channels: usize) -> Result<usize, EncodeError> {
    if samples.is_empty() {
        return Err(EncodeError::EmptyBuffer);
    }
    if channels == 0 || !samples.len().is_multiple_of(channels) {
        return Err(EncodeError::BadFrame {
            samples: samples.len(),
            channels,
        });
    }
    Ok(samples.len() / channels)
}

#[cfg(test)]
mod tests {
    use super::preflight_headroom;
    use crate::encode::Format;

    #[test]
    fn the_required_gain_brings_the_peak_to_full_scale() -> Result<(), String> {
        let loud = [3.0_f32, -1.0];
        let report = preflight_headroom(Format::FlacI16, [&loud[..]]).map_err(|e| e.to_string())?;
        if (report.required_gain() - 1.0 / 3.0).abs() >= 1e-6 {
            return Err(format!("the required gain is {}", report.required_gain()));
        }
        Ok(())
    }

    #[test]
    fn a_fitting_export_needs_no_gain() -> Result<(), String> {
        let quiet = [0.5_f32, -0.5];
        let report =
            preflight_headroom(Format::FlacI16, [&quiet[..]]).map_err(|e| e.to_string())?;
        if !report.fits() {
            return Err("a quiet export does not fit".to_owned());
        }
        if (report.required_gain() - 1.0).abs() >= f32::EPSILON {
            return Err(format!("the required gain is {}", report.required_gain()));
        }
        Ok(())
    }

    #[test]
    fn a_float_format_never_needs_a_gain() -> Result<(), String> {
        let loud = [20.0_f32, -20.0];
        let report = preflight_headroom(Format::WavF32, [&loud[..]]).map_err(|e| e.to_string())?;
        if !report.fits() {
            return Err("a float export does not fit".to_owned());
        }
        if (report.required_gain() - 1.0).abs() >= f32::EPSILON {
            return Err(format!("the required gain is {}", report.required_gain()));
        }
        Ok(())
    }

    #[test]
    fn no_output_fails() {
        let empty: [&[f32]; 0] = [];
        let error = preflight_headroom(Format::FlacI16, empty);
        assert!(matches!(
            error,
            Err(crate::encode::EncodeError::EmptyBuffer)
        ));
    }
}
