//! Property tests for the staged float file, and for the integer conversion.
//!
//! The example tests in `tests/encode.rs` pin the named cases. The staged float
//! file is the intermediate that every later stage reads, so the export needs
//! its round trip to hold for any sample and not only for six fixed ones.

use std::path::PathBuf;

use mod2stems::encode::EncodeError;
use mod2stems::encode::float_file;
use proptest::collection::vec;
use proptest::prelude::any;
use proptest::prelude::prop_assert_eq;
use proptest::proptest;
use proptest::test_runner::Config;
use proptest::test_runner::TestCaseError;

/// The output rate that the tests use.
const RATE: i32 = 48_000;

proptest! {
    #![proptest_config(Config::with_cases(64))]

    /// Every sample survives the staged file, which holds a float format.
    ///
    /// The comparison uses the bits of each sample, because a NaN is not equal
    /// to itself and the renderer can write one.
    #[test]
    fn the_staged_float_file_keeps_every_bit(
        frames in 1_usize..64,
        values in vec(any::<f32>(), 128),
    ) {
        let samples: Vec<f32> = (0..frames)
            .flat_map(|frame| [values[frame], values[frame + 64]])
            .collect();
        let path = path("round-trip")?;
        float_file::write(RATE, &samples, &path)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        let read = float_file::read(&path)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        let before: Vec<u32> = samples.iter().map(|sample| sample.to_bits()).collect();
        let after: Vec<u32> = read.samples.iter().map(|sample| sample.to_bits()).collect();
        prop_assert_eq!(after, before);
    }

}

/// An empty render is refused, so no stage writes a file with no frame.
#[test]
fn an_empty_render_is_refused() -> Result<(), String> {
    let path = path("empty").map_err(|error| error.to_string())?;
    match float_file::write(RATE, &[], &path) {
        Err(EncodeError::EmptyBuffer) => Ok(()),
        other => Err(format!("an empty buffer gives {other:?}")),
    }
}

/// Returns a path under the test target directory.
fn path(name: &str) -> Result<PathBuf, TestCaseError> {
    let base = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("float_file");
    std::fs::create_dir_all(&base).map_err(|error| {
        TestCaseError::fail(format!("cannot create {}: {error}", base.display()))
    })?;
    Ok(base.join(format!("{name}-{}.wav", std::process::id())))
}
