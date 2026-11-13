//! The output channel layout, and the mono downmix.
//!
//! Stereo is the default and keeps the renderer's panning and stereo effects.
//! Mono is the arithmetic mean of the rendered left and right samples, applied
//! the same way to stems and mixes (`docs/design.md` §4.3).
//!
//! Mono is lossy. Out-of-phase content cancels, and a stereo placement cannot
//! be recovered from it. The layout is therefore a caller's explicit choice,
//! and it is not a pan-neutral setting.

use crate::stems::StemError;
use crate::stems::timeline::frames_of;

/// The channel layout of one export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChannelLayout {
    /// Two channels, as the renderer produces them.
    #[default]
    Stereo,
    /// One channel, as the mean of the rendered pair.
    Mono,
}

/// Converts interleaved stereo samples to the named layout.
///
/// # Errors
///
/// Returns [`StemError::NotStereo`] when the input holds a half frame.
pub fn downmix(layout: ChannelLayout, samples: &[f32]) -> Result<Vec<f32>, StemError> {
    frames_of(samples)?;
    match layout {
        ChannelLayout::Stereo => Ok(samples.to_vec()),
        ChannelLayout::Mono => Ok(samples
            .as_chunks::<2>()
            .0
            .iter()
            .map(|[left, right]| (left + right) / 2.0)
            .collect()),
    }
}
