//! The output container, and the integer range that each one holds.
//!
//! Float WAV is the default and keeps every finite sample. Integer FLAC is the
//! alternative, and its range is the reason that the export preflights the
//! level before it encodes anything. Design: `docs/design.md` §5.1.

/// One output container and its sample encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    /// A WAV file of 32-bit IEEE float samples, holding a sample above full
    /// scale without clipping.
    WavF32,
    /// A FLAC file of 16-bit integer samples.
    FlacI16,
    /// A FLAC file of 24-bit integer samples.
    FlacI24,
}

impl Format {
    /// Returns the file extension that the format uses.
    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            Self::WavF32 => "wav",
            Self::FlacI16 | Self::FlacI24 => "flac",
        }
    }

    /// Returns the bits per sample that the format stores.
    #[must_use]
    pub fn bits(self) -> usize {
        match self {
            Self::WavF32 => 32,
            Self::FlacI16 => 16,
            Self::FlacI24 => 24,
        }
    }

    /// Returns true when the format stores integer samples.
    ///
    /// An integer output needs a range check before the encode, because the
    /// conversion cannot hold a sample outside it (`docs/design.md` §5.1).
    #[must_use]
    pub fn is_integer(self) -> bool {
        !matches!(self, Self::WavF32)
    }

    /// Returns the largest sample magnitude that the format holds.
    ///
    /// A float format has no such bound, so the value is infinity and every
    /// finite sample fits it.
    #[must_use]
    pub fn peak(self) -> f32 {
        match self {
            Self::WavF32 => f32::INFINITY,
            // The full scale of a signed integer format is the largest positive
            // value. The negative side reaches one step further, and the
            // quantization clamps to it, so the smaller bound is the one that
            // decides whether a sample fits.
            Self::FlacI16 | Self::FlacI24 => 1.0,
        }
    }

    /// Returns the largest magnitude that a summed set of files can differ by.
    ///
    /// A sum of integer-encoded stems differs from an integer-encoded
    /// reference by at most one step of the format, because each sample rounds
    /// once (`docs/design.md` §5.1). A float file rounds nothing, so its
    /// tolerance is zero.
    #[must_use]
    pub fn tolerance(self) -> f32 {
        match self {
            Self::WavF32 => 0.0,
            Self::FlacI16 => 1.0 / 32_768.0,
            Self::FlacI24 => 1.0 / 8_388_608.0,
        }
    }

    /// Returns the scale that converts a unit-range sample to an integer.
    #[must_use]
    pub(crate) fn scale(self) -> f32 {
        match self {
            Self::WavF32 => 1.0,
            Self::FlacI16 => 32_767.0,
            Self::FlacI24 => 8_388_607.0,
        }
    }
}

impl std::fmt::Display for Format {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WavF32 => write!(formatter, "float WAV"),
            Self::FlacI16 => write!(formatter, "16-bit FLAC"),
            Self::FlacI24 => write!(formatter, "24-bit FLAC"),
        }
    }
}
