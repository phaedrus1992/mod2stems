//! The frame shape check that the encoder runs before it writes.
//!
//! The integer path cannot hold a sample outside its range, and it never clips
//! silently. The export measures every requested output in its own preflight,
//! because that step reads the staged renders. Design: `docs/design.md` §5.1.

use crate::encode::EncodeError;

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
