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

/// The channel layout of one export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChannelLayout {
    /// Two channels, as the renderer produces them.
    #[default]
    Stereo,
    /// One channel, as the mean of the rendered pair.
    Mono,
}

impl ChannelLayout {
    /// Returns the number of channels that the layout writes per frame.
    #[must_use]
    pub fn channels(self) -> usize {
        match self {
            Self::Stereo => 2,
            Self::Mono => 1,
        }
    }
}

/// Converts interleaved stereo samples to the named layout.
///
/// # Errors
///
/// Returns [`StemError::NotStereo`] when the input holds a half frame.
pub fn downmix(layout: ChannelLayout, samples: &[f32]) -> Result<Vec<f32>, StemError> {
    if !samples.len().is_multiple_of(2) {
        return Err(StemError::NotStereo {
            samples: samples.len(),
        });
    }
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

#[cfg(test)]
mod tests {
    use super::ChannelLayout;
    use super::downmix;

    #[test]
    fn the_layout_reports_its_channel_count() {
        assert_eq!(ChannelLayout::Stereo.channels(), 2);
        assert_eq!(ChannelLayout::Mono.channels(), 1);
    }

    #[test]
    fn an_empty_render_downmixes_to_nothing() {
        assert_eq!(downmix(ChannelLayout::Mono, &[]).ok(), Some(Vec::new()));
    }

    #[test]
    fn a_cancelling_pair_gives_zero() {
        // Out-of-phase content cancels, which is why mono is documented as
        // lossy rather than pan-neutral.
        assert_eq!(
            downmix(ChannelLayout::Mono, &[1.0, -1.0]).ok(),
            Some(vec![0.0])
        );
    }
}
