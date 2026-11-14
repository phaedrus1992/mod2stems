//! The error type of the encoder.

use std::error::Error;
use std::fmt;
use std::path::Path;
use std::path::PathBuf;

/// One failed encode.
#[derive(Debug, Clone, PartialEq)]
pub enum EncodeError {
    /// The buffer holds no frame.
    EmptyBuffer,
    /// The buffer holds no whole number of frames.
    BadFrame {
        /// The number of samples in the buffer.
        samples: usize,
        /// The number of samples that one frame holds.
        channels: usize,
    },
    /// The gain is zero, negative, or not finite.
    BadGain {
        /// The gain that the caller asked for.
        gain: f32,
    },
    /// The sample rate is negative or too large for the container.
    BadRate {
        /// The rate that the caller resolved.
        rate: i32,
    },
    /// A sample is not finite.
    ///
    /// A NaN or an infinity has no integer representation, and a float file
    /// that carries one breaks every later sum without a message
    /// (`docs/design.md` §5.1).
    NotFinite {
        /// The index of the sample in the buffer.
        index: usize,
        /// The value that the buffer holds.
        value: f32,
    },
    /// A gained sample does not fit the integer range of the format.
    ///
    /// The integer path never clips silently, so the export fails and the
    /// caller chooses a common attenuation (`docs/design.md` §5.1).
    OverRange {
        /// The index of the sample in the buffer.
        index: usize,
        /// The gained value that the range cannot hold.
        value: f32,
        /// The format whose range the value exceeds.
        format: &'static str,
        /// The largest value that the format holds.
        highest: f32,
    },
    /// The file cannot be written, or the stream cannot be finished.
    Write {
        /// The path that the caller named.
        path: PathBuf,
        /// The reason from the writer.
        detail: String,
    },
    /// Two level measurements checked against different formats.
    ///
    /// The peaks of two formats are not comparable, so a merged measurement
    /// would report a range that no output used.
    FormatMismatch,
    /// The FLAC encoder rejected the stream, or gave a stream of another length.
    Flac {
        /// The reason from the encoder.
        detail: String,
    },
    /// The FLAC encoder configuration failed its own check, or the reference
    /// code was not part of the file.
    EncoderConfig {
        /// The reason from the checker.
        detail: String,
    },
}

impl EncodeError {
    /// Returns the write error for one path and one writer error.
    pub(crate) fn write(path: &Path, error: &dyn fmt::Display) -> Self {
        Self::Write {
            path: path.to_path_buf(),
            detail: error.to_string(),
        }
    }
}

impl fmt::Display for EncodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyBuffer => write!(
                formatter,
                "Cannot encode an empty buffer: it holds no audio frame. Render at least one \
                 frame, or skip the output."
            ),
            Self::BadFrame { samples, channels } => write!(
                formatter,
                "Cannot encode a buffer of {samples} samples as {channels} channel(s): each frame \
                 must hold a whole channel set. Drop the last samples, or choose another channel \
                 layout."
            ),
            Self::BadGain { gain } => write!(
                formatter,
                "Cannot encode with a gain of {gain}: the common gain must be finite and \
                 positive. Set a gain above zero, or leave the gain unset."
            ),
            Self::BadRate { rate } => write!(
                formatter,
                "Cannot encode at {rate} Hz: the sample rate must be positive and fit the \
                 container. Resolve the rate through the profile request."
            ),
            Self::NotFinite { index, value } => write!(
                formatter,
                "Cannot encode sample {index} of value {value}: the export promises finite audio. \
                 Fix the render, because a NaN or an infinity has no integer form."
            ),
            Self::OverRange {
                index,
                value,
                format,
                highest,
            } => write!(
                formatter,
                "Cannot encode sample {index} of value {value} as {format}: the format holds at \
                 most {highest}. Choose a common gain that brings every output inside the range, \
                 or export as float WAV."
            ),
            Self::Write { path, detail } => write!(
                formatter,
                "Cannot write {}: {detail}. Check that the parent directory exists and is \
                 writable.",
                path.display()
            ),
            Self::FormatMismatch => write!(
                formatter,
                "Cannot merge the level measurements: they checked against different formats. \
                 Measure every output against the format that the export writes, then merge the \
                 results."
            ),
            Self::Flac { detail } => write!(
                formatter,
                "Cannot encode the FLAC stream: {detail}. Check the bit depth and the sample \
                 range of the buffer."
            ),
            Self::EncoderConfig { detail } => write!(
                formatter,
                "Cannot build the FLAC encoder: {detail}. Report this, because the encoder \
                 rejected its own default configuration."
            ),
        }
    }
}

impl Error for EncodeError {}
