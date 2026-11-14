//! The encoder that writes one rendered buffer to one file.
//!
//! The float path keeps every finite sample, including a sample above nominal
//! full scale. The integer path checks the range first and fails instead of
//! clipping. One common gain scales every output of an export, and no output is
//! normalized on its own. Design: `docs/design.md` §5.1.
//!
//! PCM stays in float until the final integer conversion. Nothing in this
//! module reads intermediate audio back through an integer reader.

pub mod error;
pub mod float_file;
pub mod format;
pub mod gain;
pub mod headroom;
pub mod quantize;

pub use error::EncodeError;
pub use format::Format;
pub use gain::Gain;
pub use headroom::Headroom;
pub use headroom::preflight_headroom;
pub use headroom::preflight_headroom_for;
pub use quantize::Dither;

use std::path::Path;

/// The settings that one output file uses.
///
/// An `Encoding` holds the container, the sample rate, and the channel count,
/// which must be resolved already. [`Self::with_gain`] and [`Self::with_dither`]
/// add the two policies that the export records. The common gain is a property
/// of the export, not of one file, so the caller passes the same value to every
/// file.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Encoding {
    format: Format,
    rate: i32,
    channels: usize,
    gain: Gain,
    dither: Dither,
}

impl Encoding {
    /// Returns a stereo encoding with no gain and no dither.
    ///
    /// `rate` must already be a resolved output rate. The constructor does not
    /// check it, because the profile request checks it against the renderer
    /// before any render starts, and the export shares that value.
    #[must_use]
    pub fn new(format: Format, rate: i32) -> Self {
        Self {
            format,
            rate,
            channels: 2,
            gain: Gain::unity(),
            dither: Dither::None,
        }
    }

    /// Returns the encoding with another channel count.
    ///
    /// The count is the number of samples in one frame. A mono export writes
    /// one sample per frame, and the caller downmixes the render first.
    #[must_use]
    pub fn with_channels(self, channels: usize) -> Self {
        Self { channels, ..self }
    }

    /// Returns the channel count.
    #[must_use]
    pub fn channels(self) -> usize {
        self.channels
    }

    /// Returns the encoding with a common gain.
    ///
    /// # Errors
    ///
    /// Returns [`EncodeError::BadGain`] when the gain is zero, negative, or not
    /// finite. A zero gain silences the export and a negative gain inverts it,
    /// and neither is an attenuation.
    pub fn with_gain(self, gain: f32) -> Result<Self, EncodeError> {
        Ok(Self {
            gain: Gain::new(gain)?,
            ..self
        })
    }

    /// Returns the encoding with a dither policy for the integer path.
    ///
    /// The float path ignores the policy, because it writes no integer sample.
    #[must_use]
    pub fn with_dither(self, dither: Dither) -> Self {
        Self { dither, ..self }
    }

    /// Returns the container.
    #[must_use]
    pub fn format(self) -> Format {
        self.format
    }

    /// Returns the sample rate.
    #[must_use]
    pub fn rate(self) -> i32 {
        self.rate
    }

    /// Returns the common gain.
    #[must_use]
    pub fn gain(self) -> Gain {
        self.gain
    }

    /// Returns the dither policy.
    #[must_use]
    pub fn dither(self) -> Dither {
        self.dither
    }
}

/// Writes one buffer to one file under the settings of `encoding`.
///
/// The call applies the common gain, checks every sample, and writes the file.
/// It does not create a parent directory and it does not stage the file, so the
/// caller owns both.
///
/// # Errors
///
/// Returns [`EncodeError::EmptyBuffer`] when the buffer holds no frame, and
/// [`EncodeError::BadFrame`] when it holds no whole frame. Returns
/// [`EncodeError::NotFinite`] for a sample that is not finite, because the
/// export promises finite audio. Returns [`EncodeError::OverRange`] when an
/// integer output cannot hold a gained sample. Returns
/// [`EncodeError::Write`] when the file or the stream fails.
pub fn encode(encoding: &Encoding, samples: &[f32], path: &Path) -> Result<(), EncodeError> {
    let frames = headroom::check_shape(samples, encoding.channels)?;
    let gained = gain::apply(encoding.gain, samples)?;
    match encoding.format {
        Format::WavF32 => write_wav_f32(encoding, &gained, path),
        Format::FlacI16 | Format::FlacI24 => {
            let integers = quantize::to_integers(encoding, &gained)?;
            write_flac(encoding, &integers, frames, path)
        }
    }
}

/// Writes a float WAV file.
fn write_wav_f32(encoding: &Encoding, samples: &[f32], path: &Path) -> Result<(), EncodeError> {
    let rate = encoding.rate;
    let spec = hound::WavSpec {
        channels: u16::try_from(encoding.channels).map_err(|_| EncodeError::BadFrame {
            samples: samples.len(),
            channels: encoding.channels,
        })?,
        sample_rate: u32::try_from(rate).map_err(|_| EncodeError::BadRate { rate })?,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let write = || -> Result<(), EncodeError> {
        let mut writer = hound::WavWriter::create(path, spec)
            .map_err(|error| EncodeError::write(path, &error))?;
        for sample in samples {
            writer
                .write_sample(*sample)
                .map_err(|error| EncodeError::write(path, &error))?;
        }
        writer
            .finalize()
            .map_err(|error| EncodeError::write(path, &error))
    };
    write()
}

/// Writes a FLAC stream.
fn write_flac(
    encoding: &Encoding,
    integers: &[i32],
    frames: usize,
    path: &Path,
) -> Result<(), EncodeError> {
    use flacenc::component::BitRepr;
    use flacenc::error::Verify;

    let bits = encoding.format.bits();
    let rate = usize::try_from(encoding.rate).map_err(|_| EncodeError::BadRate {
        rate: encoding.rate,
    })?;
    let config = flacenc::config::Encoder::default()
        .into_verified()
        .map_err(|(_, error)| EncodeError::EncoderConfig {
            detail: format!("{error}"),
        })?;
    let source = flacenc::source::MemSource::from_samples(integers, encoding.channels, bits, rate);
    let stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
        .map_err(|error| EncodeError::Flac {
            detail: error.to_string(),
        })?;
    // A stream that holds fewer frames than the buffer loses audio, and one
    // that holds more repeats it. Either way the file is not this stem.
    let written = stream.stream_info().total_samples();
    if written != frames {
        return Err(EncodeError::Flac {
            detail: format!("the stream holds {written} frames but the buffer holds {frames}"),
        });
    }
    let mut sink = flacenc::bitsink::ByteSink::new();
    stream.write(&mut sink).map_err(|error| EncodeError::Flac {
        detail: error.to_string(),
    })?;
    std::fs::write(path, sink.as_slice()).map_err(|error| EncodeError::write(path, &error))
}

#[cfg(test)]
mod tests {
    use super::Encoding;
    use super::Format;

    #[test]
    fn an_encoding_starts_at_unity_gain_and_no_dither() {
        let encoding = Encoding::new(Format::WavF32, 48_000);
        assert_eq!(encoding.gain().value(), 1.0);
        assert_eq!(encoding.dither(), super::Dither::None);
        assert_eq!(encoding.rate(), 48_000);
        assert_eq!(encoding.format(), Format::WavF32);
    }
}
