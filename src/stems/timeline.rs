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
///
/// A `Timeline` holds its length in stereo frames. A mono buffer has one sample
/// per frame, so it names twice as many frames as the render that it came from.
/// Downmix a render only after the timeline pads it, or the timeline reports the
/// mono buffer as another render's length.
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
            let frames = frames_of(samples)?;
            longest = Some(longest.map_or(frames, |found: usize| found.max(frames)));
        }
        // The zero case fails, because an export with no render has no length
        // to align to and every stem of it would be empty.
        match longest {
            Some(frames) if frames > 0 => Ok(Self { frames }),
            _ => Err(StemError::EmptyTimeline),
        }
    }

    /// Builds a timeline from the frame count that the caller measured.
    ///
    /// An export that stages its renders in temporary files measures each
    /// render without holding it in memory. The caller must pass the longest
    /// frame count of every render that the export writes, the unmuted render
    /// included, because the timeline is what aligns them.
    ///
    /// `limit` is the frame count that one render can reach, so a count above
    /// it names a padding buffer that no render produced.
    ///
    /// # Errors
    ///
    /// Returns [`StemError::EmptyTimeline`] when the count is zero, and
    /// [`StemError::TimelinePastLimit`] when the count is above `limit`.
    pub fn from_frame_count(frames: usize, limit: usize) -> Result<Self, StemError> {
        if frames == 0 {
            return Err(StemError::EmptyTimeline);
        }
        if frames > limit {
            return Err(StemError::TimelinePastLimit { frames, limit });
        }
        Ok(Self { frames })
    }

    /// Returns the length of the common timeline, in frames.
    #[must_use]
    pub fn frames(self) -> usize {
        self.frames
    }

    /// Returns the frames that one render needs added to reach the timeline.
    ///
    /// The caller reports this value before it pads, so it sees a timing
    /// difference before the padding hides it.
    ///
    /// # Errors
    ///
    /// Returns [`StemError::NotStereo`] when the render holds a half frame, and
    /// [`StemError::RenderPastTimeline`] when the render is longer than the
    /// timeline. A saturating subtraction would report both cases as zero, and
    /// a caller would read a divergent render as an aligned one.
    pub fn padding_for(self, samples: &[f32]) -> Result<usize, StemError> {
        let frames = frames_of(samples)?;
        if frames > self.frames {
            return Err(StemError::RenderPastTimeline {
                frames,
                timeline: self.frames,
            });
        }
        Ok(self.frames - frames)
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
        let frames = frames_of(samples)?;
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

/// Returns the frame count of a render, and rejects a half frame.
///
/// The samples are interleaved stereo, so a count that is not a multiple of two
/// cannot name a whole frame. This is the one copy of the rule; `downmix` calls
/// it too, so the two cannot drift apart.
pub(crate) fn frames_of(samples: &[f32]) -> Result<usize, StemError> {
    if !samples.len().is_multiple_of(2) {
        return Err(StemError::NotStereo {
            samples: samples.len(),
        });
    }
    Ok(samples.len() / 2)
}
