//! The common timeline that every stem of one export shares.
//!
//! Each render starts at the module origin and stops where the library stops
//! it, so two renders of one module can differ in length. The common end is the
//! longest completed render, and a shorter render is zero-padded at the tail so
//! every track lines up (`docs/design.md` §4.2).
//!
//! The end is the library song end. No stock control drains a voice that still
//! rings at that point, so the timeline stops there and the export states the
//! limit.

use crate::stems::StemError;

/// The frame count that every stem and mix of one export spans.
///
/// The value is built from the completed renders, so it can only name a length
/// that some render reached. A render longer than the timeline is a defect in
/// the caller, and [`Self::pad`] refuses it rather than truncating audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timeline {
    frames: usize,
}

impl Timeline {
    /// Builds a timeline from the completed renders of one export.
    ///
    /// The caller passes every render that the timeline must hold, the unmuted
    /// render included.
    ///
    /// # Errors
    ///
    /// Returns [`StemError::EmptyTimeline`] when the caller passes no render,
    /// and [`StemError::NotStereo`] when a render holds a half frame.
    pub fn from_renders<'a, I>(renders: I) -> Result<Self, StemError>
    where
        I: IntoIterator<Item = &'a [f32]>,
    {
        let mut longest: Option<usize> = None;
        for samples in renders {
            frames_of(samples)?;
            let frames = samples.len() / 2;
            longest = Some(longest.map_or(frames, |found: usize| found.max(frames)));
        }
        // The zero case fails, because an export with no render has no length
        // to align to and every stem of it would be empty.
        match longest {
            Some(frames) if frames > 0 => Ok(Self { frames }),
            _ => Err(StemError::EmptyTimeline),
        }
    }

    /// Returns the length of the common timeline, in frames.
    #[must_use]
    pub fn frames(self) -> usize {
        self.frames
    }

    /// Returns the frames that one render needs added to reach the timeline.
    #[must_use]
    pub fn padding_for(self, samples: &[f32]) -> usize {
        self.frames.saturating_sub(samples.len() / 2)
    }

    /// Returns one render, zero-padded at the tail to the common length.
    ///
    /// The head is never touched, so a render that starts with silence keeps
    /// that silence and every track starts at the same origin.
    ///
    /// # Errors
    ///
    /// Returns [`StemError::NotStereo`] when the render holds a half frame, and
    /// [`StemError::RenderPastTimeline`] when the render is longer than the
    /// timeline. Padding cannot shorten a render, because a shorter common end
    /// would cut audio that the library played.
    pub fn pad(self, samples: &[f32]) -> Result<Vec<f32>, StemError> {
        frames_of(samples)?;
        let frames = samples.len() / 2;
        if frames > self.frames {
            return Err(StemError::RenderPastTimeline {
                frames,
                timeline: self.frames,
            });
        }
        let mut padded = Vec::with_capacity(self.frames.saturating_mul(2));
        padded.extend_from_slice(samples);
        padded.resize(self.frames.saturating_mul(2), 0.0);
        Ok(padded)
    }
}

/// Rejects a sample count that is not a whole number of stereo frames.
fn frames_of(samples: &[f32]) -> Result<(), StemError> {
    if samples.len().is_multiple_of(2) {
        return Ok(());
    }
    Err(StemError::NotStereo {
        samples: samples.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::Timeline;

    #[test]
    fn the_longest_render_sets_the_length() {
        let short = [0.0_f32; 2];
        let long = [0.0_f32; 6];
        let timeline = Timeline::from_renders([&short[..], &long[..]]);
        assert_eq!(timeline.map(Timeline::frames).ok(), Some(3));
    }

    #[test]
    fn a_render_at_the_length_needs_no_padding() {
        let only = [0.0_f32; 4];
        let timeline = Timeline::from_renders([&only[..]]);
        assert_eq!(timeline.map(|t| t.padding_for(&only)).ok(), Some(0));
    }
}
