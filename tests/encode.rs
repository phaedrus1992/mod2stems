//! Tests the encoder that writes one rendered buffer to one file.
//!
//! The checks cover the format selection, the float path, the integer
//! preflight, the common gain, and the quantization tolerance.

use std::fmt::Debug;
use std::path::Path;
use std::path::PathBuf;

use mod2stems::encode::Dither;
use mod2stems::encode::EncodeError;
use mod2stems::encode::Encoding;
use mod2stems::encode::Format;
use mod2stems::encode::encode;
use mod2stems::encode::float_file;
use mod2stems::encode::preflight_headroom;

/// The output rate that the tests use.
const RATE: i32 = 48_000;

/// The number of frames in one second at [`RATE`].
const FRAMES: usize = 48_000;

/// The phase step of the test tone, in radians per frame.
const TONE_STEP: f32 = 440.0 * std::f32::consts::TAU / 48_000.0;

/// Returns a stereo buffer of one second of a 440 Hz tone.
fn tone(amplitude: f32) -> Vec<f32> {
    let mut out = Vec::with_capacity(FRAMES.saturating_mul(2));
    let mut phase = 0.0_f32;
    for _ in 0..FRAMES {
        let value = phase.sin() * amplitude;
        out.extend_from_slice(&[value, value]);
        phase += TONE_STEP;
    }
    out
}

/// Returns the default float encoding.
fn float_encoding() -> Encoding {
    Encoding::new(Format::WavF32, RATE)
}

/// Returns an error that names `what` when `condition` is false.
fn require(condition: bool, what: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(what.to_owned())
    }
}

/// Returns an error that names both values when they differ.
fn require_eq<T: Debug + PartialEq>(actual: &T, expected: &T, what: &str) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{what}: got {actual:?}, expected {expected:?}"))
    }
}

/// Returns an error that names both values when two small values differ.
fn require_val<T: Copy + Debug + PartialEq>(
    actual: T,
    expected: T,
    what: &str,
) -> Result<(), String> {
    require_eq(&actual, &expected, what)
}

#[test]
fn float_keeps_a_sample_above_full_scale() -> Result<(), String> {
    // The float format keeps finite samples above nominal full scale. A clip
    // here would lose the headroom that the design promises
    // (docs/design.md §5.1).
    let dir = tempdir()?;
    let path = dir.join("stem.wav");
    let samples = vec![1.5_f32, 1.5, -1.25, -1.25];
    encode(&float_encoding(), &samples, &path).map_err(|error| error.to_string())?;
    require_eq(
        &read_wav(&path)?,
        &samples,
        "the float file holds the input",
    )
}

#[test]
fn float_applies_the_common_gain() -> Result<(), String> {
    let dir = tempdir()?;
    let path = dir.join("stem.wav");
    let samples = vec![0.5_f32, -0.5, 0.25, -0.25];
    let encoding = float_encoding().with_gain(0.5).map_err(|e| e.to_string())?;
    encode(&encoding, &samples, &path).map_err(|error| error.to_string())?;
    require_eq(
        &read_wav(&path)?,
        &vec![0.25_f32, -0.25, 0.125, -0.125],
        "the gain halves every sample",
    )
}

#[test]
fn float_rejects_a_gain_that_is_not_positive() -> Result<(), String> {
    // A zero or negative gain silences the export or inverts it, and neither
    // is what "attenuation" means. The error names the setting and the fix.
    let error = match float_encoding().with_gain(0.0) {
        Ok(_) => return Err("a zero gain must fail".to_owned()),
        Err(error) => error.to_string(),
    };
    require(
        error.contains("gain") && error.contains("positive"),
        &format!("the error must name the setting and the fix, got {error:?}"),
    )
}

#[test]
fn a_gain_above_one_is_accepted() -> Result<(), String> {
    // The gain is an attenuation for the integer path, but the float path also
    // accepts a boost, and the preflight is what decides whether it fits.
    float_encoding()
        .with_gain(2.0)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[test]
fn preflight_reports_every_output_over_the_range() -> Result<(), String> {
    // The preflight names the peak and how many outputs exceed the range, so
    // the caller chooses a common attenuation with the cost in front of it.
    let loud = vec![0.5_f32, 1.5, -3.0, 0.0];
    let quiet = vec![0.1_f32, -0.1, 0.1, -0.1];
    let report = preflight_headroom(
        Format::FlacI16,
        [loud.as_slice(), quiet.as_slice(), loud.as_slice()],
    )
    .map_err(|error| error.to_string())?;
    require(!report.fits(), "a sample of 3.0 is over 16-bit range")?;
    require_val(report.peak(), 3.0, "the peak")?;
    require_val(report.outputs_over(), 2, "the outputs over the range")?;
    require_val(report.outputs(), 3, "the output count")
}

#[test]
fn preflight_passes_a_signal_inside_the_integer_range() -> Result<(), String> {
    let quiet = vec![0.5_f32, -0.5, 0.25, -0.25];
    let report =
        preflight_headroom(Format::FlacI16, [quiet.as_slice()]).map_err(|e| e.to_string())?;
    require(report.fits(), "a signal below full scale fits")?;
    require_val(report.outputs_over(), 0, "the outputs over the range")
}

#[test]
fn preflight_needs_no_headroom_for_float() -> Result<(), String> {
    // A float output holds any finite sample, so the integer range never
    // rejects it (docs/design.md §5.1).
    let loud = vec![9.0_f32, -9.0];
    let report =
        preflight_headroom(Format::WavF32, [loud.as_slice()]).map_err(|e| e.to_string())?;
    require(report.fits(), "a float output has no range to exceed")?;
    require_val(report.outputs_over(), 0, "the outputs over the range")
}

#[test]
fn flac_writes_a_readable_file() -> Result<(), String> {
    let dir = tempdir()?;
    let path = dir.join("stem.flac");
    let encoding = Encoding::new(Format::FlacI16, RATE);
    encode(&encoding, &tone(0.5), &path).map_err(|error| error.to_string())?;
    let bytes = std::fs::read(&path).map_err(|error| error.to_string())?;
    require_eq(
        &bytes.first_chunk::<4>(),
        &Some(b"fLaC"),
        "a FLAC stream starts with its marker",
    )?;
    require(bytes.len() > 4, "the file holds a stream")
}

#[test]
fn flac_refuses_a_sample_that_needs_a_bigger_range() -> Result<(), String> {
    // The integer path never clips silently. A sample past the range fails,
    // and the error names the value and the fix.
    let dir = tempdir()?;
    let path = dir.join("stem.flac");
    let samples = vec![0.0_f32, 2.0, 0.0, 0.0];
    let encoding = Encoding::new(Format::FlacI16, RATE);
    let error = match encode(&encoding, &samples, &path) {
        Ok(()) => return Err("an over-range sample must fail".to_owned()),
        Err(error) => error,
    };
    require(
        matches!(error, EncodeError::OverRange { .. }),
        &format!("the error must be the range error, got {error:?}"),
    )?;
    require(
        error.to_string().contains("gain"),
        "the error must name the fix",
    )
}

#[test]
fn flac_keeps_a_gained_signal_inside_the_range() -> Result<(), String> {
    let dir = tempdir()?;
    let path = dir.join("stem.flac");
    let samples = vec![1.8_f32, -1.8, 0.0, 0.0];
    let encoding = Encoding::new(Format::FlacI16, RATE)
        .with_gain(0.5)
        .map_err(|e| e.to_string())?;
    encode(&encoding, &samples, &path).map_err(|error| error.to_string())?;
    require(path.exists(), "the file exists after a good encode")
}

#[test]
fn flac_refuses_a_non_finite_sample() -> Result<(), String> {
    // A NaN or an infinity reaches the integer conversion as an undefined
    // value. The export fails instead of writing a sample it cannot name.
    let dir = tempdir()?;
    let path = dir.join("stem.flac");
    let encoding = Encoding::new(Format::FlacI16, RATE);
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let samples = vec![0.0_f32, bad, 0.0, 0.0];
        let error = match encode(&encoding, &samples, &path) {
            Ok(()) => return Err(format!("a sample of {bad} must fail")),
            Err(error) => error,
        };
        require(
            matches!(error, EncodeError::NotFinite { .. }),
            &format!("the error must name the non-finite value, got {error:?}"),
        )?;
    }
    Ok(())
}

#[test]
fn float_refuses_a_non_finite_sample() -> Result<(), String> {
    // A float file can hold a NaN, but the export promises finite samples, and
    // a NaN in a stem breaks every later sum without a message.
    let dir = tempdir()?;
    let path = dir.join("stem.wav");
    let samples = vec![f32::NAN, 0.0];
    let error = match encode(&float_encoding(), &samples, &path) {
        Ok(()) => return Err("a NaN must fail".to_owned()),
        Err(error) => error,
    };
    require(
        matches!(error, EncodeError::NotFinite { .. }),
        &format!("the error must name the non-finite value, got {error:?}"),
    )
}

#[test]
fn a_buffer_with_a_half_frame_fails() -> Result<(), String> {
    let dir = tempdir()?;
    let path = dir.join("stem.wav");
    let error = match encode(&float_encoding(), &[0.0, 0.0, 0.0], &path) {
        Ok(()) => return Err("a half frame must fail".to_owned()),
        Err(error) => error,
    };
    require(
        matches!(error, EncodeError::BadFrame { .. }),
        &format!("the error must be the stereo error, got {error:?}"),
    )
}

#[test]
fn an_empty_buffer_fails() -> Result<(), String> {
    let dir = tempdir()?;
    let path = dir.join("stem.wav");
    let error = match encode(&float_encoding(), &[], &path) {
        Ok(()) => return Err("an empty buffer must fail".to_owned()),
        Err(error) => error,
    };
    require(
        matches!(error, EncodeError::EmptyBuffer),
        &format!("the error must be the empty error, got {error:?}"),
    )
}

#[test]
fn the_tolerance_follows_the_bit_depth() {
    // A sum of quantized stems differs from the quantized reference by at most
    // one step of the format, so the tolerance is that step
    // (docs/design.md §5.1).
    let tolerance = Format::FlacI16.tolerance();
    let step = 1.0_f32 / 32_768.0;
    assert!(
        (tolerance - step).abs() < f32::EPSILON,
        "the tolerance {tolerance} does not match the step {step}"
    );
    assert_eq!(Format::WavF32.tolerance(), 0.0);
}

#[test]
fn dither_widens_the_tolerance_by_its_own_width() {
    // Dither adds noise that the undithered comparison does not expect, so the
    // tolerance must cover both the rounding step and the dither width
    // (docs/design.md §5.1). It can never shrink the tolerance.
    let step = Format::FlacI16.tolerance();
    assert_eq!(Dither::None.tolerance(Format::FlacI16), step);
    assert!(
        Dither::Tpdf.tolerance(Format::FlacI16) > step,
        "a dithered set compares within a wider tolerance, not a narrower one"
    );
}

#[test]
fn a_dithered_encode_is_reproducible() -> Result<(), String> {
    // A repeat export must give the same bytes, or the manifest hash cannot
    // name the output. The dither generator is seeded from the export.
    let dir = tempdir()?;
    let samples = tone(0.5);
    let first = dir.join("first.flac");
    let second = dir.join("second.flac");
    let encoding = Encoding::new(Format::FlacI16, RATE).with_dither(Dither::Tpdf);
    encode(&encoding, &samples, &first).map_err(|e| e.to_string())?;
    encode(&encoding, &samples, &second).map_err(|e| e.to_string())?;
    let a = std::fs::read(&first).map_err(|e| e.to_string())?;
    let b = std::fs::read(&second).map_err(|e| e.to_string())?;
    require_eq(&a, &b, "the same input gives the same bytes")
}

#[test]
fn a_staged_float_file_round_trips_every_bit() -> Result<(), String> {
    // The staged file is the one source of the final encode, so a lost bit
    // would reach the export.
    let samples = vec![0.0_f32, 1.5, -1.5, 1.0 / 3.0, -0.000_000_1, 0.25];
    let path = tempdir()?.join("round-trip.wav");
    float_file::write(RATE, &samples, &path).map_err(|e| e.to_string())?;
    let read_back = float_file::read(&path).map_err(|e| e.to_string())?;
    require_val(read_back.rate, RATE, "the staged rate")?;
    require_eq(&read_back.samples, &samples, "the staged samples")
}

#[test]
fn a_staged_half_frame_is_refused() -> Result<(), String> {
    let path = tempdir()?.join("half-frame.wav");
    let error = match float_file::write(RATE, &[0.0, 0.0, 0.0], &path) {
        Ok(()) => return Err("a half frame must fail".to_owned()),
        Err(error) => error,
    };
    require(
        matches!(error, EncodeError::BadFrame { .. }),
        &format!("the error must name the half frame, got {error:?}"),
    )
}

/// Creates an empty scratch directory for one test.
fn tempdir() -> Result<PathBuf, String> {
    let base = Path::new(env!("CARGO_TARGET_TMPDIR")).join("encode");
    std::fs::create_dir_all(&base).map_err(|error| error.to_string())?;
    let unique = format!("{}-{:?}", std::process::id(), std::thread::current().id());
    let dir = base.join(unique.replace(['(', ')', ' '], ""));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

/// Reads every sample of a float WAV file.
fn read_wav(path: &Path) -> Result<Vec<f32>, String> {
    let mut reader = hound::WavReader::open(path).map_err(|error| error.to_string())?;
    reader
        .samples::<f32>()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}
