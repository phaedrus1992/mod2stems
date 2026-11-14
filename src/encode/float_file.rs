//! The temporary float file that one export stages its audio in.
//!
//! An export measures every output before it encodes any of them, so the integer
//! range check covers the whole set. The export cannot hold every channel of a
//! long song in memory at once, so each stem goes to a float file first. A
//! 32-bit float WAV holds every f32 bit for bit, so the second pass reads the
//! exact samples that the render produced. Design: `docs/design.md` §4.2, §5.1.

use std::path::Path;

use crate::encode::EncodeError;

/// The audio of one staged float file.
#[derive(Debug, Clone, PartialEq)]
pub struct FloatAudio {
    /// The sample rate that the file states, in Hz.
    pub rate: i32,
    /// The interleaved stereo samples.
    pub samples: Vec<f32>,
}

/// Writes interleaved stereo float samples to one WAV file.
///
/// The call applies no gain and no dither. It holds the render output as the
/// renderer wrote it, because the export applies the common gain in its last
/// pass and nowhere else.
///
/// # Errors
///
/// Returns [`EncodeError::EmptyBuffer`] when the buffer holds no frame,
/// [`EncodeError::NotStereo`] when it holds a half frame, and
/// [`EncodeError::Write`] when the file fails.
pub fn write(rate: i32, samples: &[f32], path: &Path) -> Result<(), EncodeError> {
    crate::encode::headroom::check_shape(samples, 2)?;
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: u32::try_from(rate).map_err(|_| EncodeError::BadRate { rate })?,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer =
        hound::WavWriter::create(path, spec).map_err(|error| EncodeError::write(path, &error))?;
    for sample in samples {
        writer
            .write_sample(*sample)
            .map_err(|error| EncodeError::write(path, &error))?;
    }
    writer
        .finalize()
        .map_err(|error| EncodeError::write(path, &error))
}

/// Reads one interleaved stereo float WAV file.
///
/// # Errors
///
/// Returns [`EncodeError::NotStereo`] when the file holds a channel count that
/// is not two, [`EncodeError::BadRate`] when the file states no usable rate,
/// and [`EncodeError::Write`] when the file fails to read.
pub fn read(path: &Path) -> Result<FloatAudio, EncodeError> {
    let mut reader =
        hound::WavReader::open(path).map_err(|error| EncodeError::write(path, &error))?;
    let spec = reader.spec();
    if spec.sample_format != hound::SampleFormat::Float {
        return Err(EncodeError::BadFrame {
            samples: 0,
            channels: usize::from(spec.channels),
        });
    }
    let rate =
        i32::try_from(spec.sample_rate).map_err(|_| EncodeError::BadRate { rate: i32::MAX })?;
    let samples = reader
        .samples::<f32>()
        .collect::<Result<Vec<f32>, _>>()
        .map_err(|error| EncodeError::write(path, &error))?;
    Ok(FloatAudio { rate, samples })
}
