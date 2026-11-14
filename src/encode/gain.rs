//! The common gain that one export applies to every output.
//!
//! The gain is one value for the whole export, so every stem and every mix
//! scales by the same factor. It is never a per-track normalization
//! (`docs/design.md` §5.1).

use crate::encode::EncodeError;

/// The factor that every output of one export scales by.
///
/// The value is finite and positive. Unity is the default and changes nothing.
/// A value below one attenuates, which is how the export brings an integer
/// output inside its range without clipping.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gain {
    value: f32,
}

impl Gain {
    /// Returns the unity gain.
    #[must_use]
    pub const fn unity() -> Self {
        Self { value: 1.0 }
    }

    /// Returns a gain of the named factor.
    ///
    /// # Errors
    ///
    /// Returns [`EncodeError::BadGain`] when the factor is zero, negative, or
    /// not finite.
    pub fn new(value: f32) -> Result<Self, EncodeError> {
        if !value.is_finite() || value <= 0.0 {
            return Err(EncodeError::BadGain { gain: value });
        }
        Ok(Self { value })
    }

    /// Returns the factor.
    #[must_use]
    pub fn value(self) -> f32 {
        self.value
    }
}

/// Scales every sample by `gain`, and checks that each result is finite.
///
/// # Errors
///
/// Returns [`EncodeError::NotFinite`] for a sample that is not finite, before
/// the gain, or for a result that the gain pushed out of the float range.
pub(crate) fn apply(gain: Gain, samples: &[f32]) -> Result<Vec<f32>, EncodeError> {
    let mut out = Vec::with_capacity(samples.len());
    for (index, sample) in samples.iter().copied().enumerate() {
        if !sample.is_finite() {
            return Err(EncodeError::NotFinite {
                index,
                value: sample,
            });
        }
        let scaled = sample * gain.value;
        if !scaled.is_finite() {
            return Err(EncodeError::NotFinite {
                index,
                value: scaled,
            });
        }
        out.push(scaled);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::Gain;
    use super::apply;

    #[test]
    fn unity_changes_nothing() -> Result<(), String> {
        let scaled = apply(Gain::unity(), &[0.5, -2.0]).map_err(|e| e.to_string())?;
        if scaled != [0.5, -2.0] {
            return Err(format!("unity gives {scaled:?}"));
        }
        Ok(())
    }

    #[test]
    fn a_half_gain_halves_every_sample() -> Result<(), String> {
        let gain = Gain::new(0.5).map_err(|e| e.to_string())?;
        let scaled = apply(gain, &[0.5, -0.5]).map_err(|e| e.to_string())?;
        if scaled != [0.25, -0.25] {
            return Err(format!("a half gain gives {scaled:?}"));
        }
        Ok(())
    }

    #[test]
    fn a_non_finite_input_sample_fails() {
        for bad in [f32::NAN, f32::INFINITY] {
            let error = apply(Gain::unity(), &[0.0, bad]);
            assert!(
                matches!(error, Err(super::EncodeError::NotFinite { .. })),
                "a non-finite sample fails"
            );
        }
    }

    #[test]
    fn a_gain_that_overflows_the_float_range_fails() -> Result<(), String> {
        // A finite sample times a finite gain can still reach infinity, and the
        // export promises finite audio.
        let gain = Gain::new(f32::MAX).map_err(|e| e.to_string())?;
        match apply(gain, &[f32::MAX]) {
            Err(super::EncodeError::NotFinite { .. }) => Ok(()),
            other => Err(format!("an overflowing product gives {other:?}")),
        }
    }
}
