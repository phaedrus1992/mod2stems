//! The float stereo render.

use std::ops::RangeInclusive;

use crate::error::OpenmptError;
use crate::error::Result;
use crate::module::Module;

/// The sample rates that libopenmpt documents for the read functions.
///
/// The library does not enforce this range. A rate of 0 reaches a division in
/// the mixer, so this crate rejects a rate outside the range before the call.
pub const SAMPLE_RATE_RANGE: RangeInclusive<i32> = 8000..=192_000;

impl Module {
    /// Renders interleaved stereo float frames into `buf`.
    ///
    /// Returns the number of frames that the call wrote. A value of 0 means the
    /// song ended and the order list did not repeat.
    ///
    /// # Errors
    ///
    /// Returns an error when `buf.len()` is odd, when `rate` is outside
    /// [`SAMPLE_RATE_RANGE`], or when libopenmpt rejects the call.
    pub fn read_float(&mut self, rate: i32, buf: &mut [f32]) -> Result<usize> {
        if !buf.len().is_multiple_of(2) {
            return Err(OpenmptError::new(
                "render",
                format!("buffer of {} samples", buf.len()),
                "an interleaved stereo buffer needs an even length, because each frame holds two samples",
                Some("pass a buffer whose length is a multiple of 2"),
            ));
        }
        if !SAMPLE_RATE_RANGE.contains(&rate) {
            return Err(OpenmptError::new(
                "render",
                format!("rate {rate}"),
                format!(
                    "the sample rate {rate} is outside {}..={}",
                    SAMPLE_RATE_RANGE.start(),
                    SAMPLE_RATE_RANGE.end()
                ),
                Some("use a sample rate in the range that libopenmpt documents"),
            ));
        }
        let frames = buf.len() / 2;
        self.clear_error();
        // SAFETY: the handle is live. `buf` holds `frames * 2` samples, which is
        // the count that the call writes.
        let written = unsafe {
            openmpt_ext_sys::openmpt_module_read_interleaved_float_stereo(
                self.core(),
                rate,
                frames,
                buf.as_mut_ptr(),
            )
        };
        self.check_last_error("render", format!("{frames} frames at {rate} Hz"))?;
        Ok(written)
    }
}
