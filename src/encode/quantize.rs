//! The conversion to integer samples, and the dither policy.
//!
//! Quantization happens at the final integer encode only. No dither runs
//! during intermediate processing, and a float export runs none at all
//! (`docs/design.md` §5.1).
//!
//! The dither generator is seeded from a fixed value, so two exports of the
//! same input give the same bytes. A manifest hash that changed on a repeat
//! export would name nothing.

use crate::encode::EncodeError;
use crate::encode::Encoding;

/// The dither that the final integer conversion applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Dither {
    /// No dither. The conversion rounds to the nearest integer.
    #[default]
    None,
    /// Triangular probability density function dither, one zero-mean noise
    /// step wide.
    ///
    /// TPDF is the usual choice for a final integer encode, because it removes
    /// the correlation between the quantization error and the signal
    /// (`docs/design.md` §5.1).
    Tpdf,
}

impl Dither {
    /// Returns the largest magnitude that the policy can add to a sample.
    ///
    /// A TPDF step is one quantization step wide, so it cannot push a sample
    /// that fit the range past it by more than one step. The caller adds this
    /// to the tolerance.
    #[must_use]
    pub fn tolerance(self, format: crate::encode::Format) -> f32 {
        match self {
            Self::None => format.tolerance(),
            Self::Tpdf => format.tolerance().mul_add(2.0, format.tolerance()),
        }
    }
}

/// Converts a gained float buffer to the integer samples of `encoding`.
///
/// # Errors
///
/// Returns [`EncodeError::OverRange`] when a sample does not fit the range, and
/// [`EncodeError::NotFinite`] for a sample that is not finite.
pub(crate) fn to_integers(encoding: &Encoding, samples: &[f32]) -> Result<Vec<i32>, EncodeError> {
    let format = encoding.format();
    let mut out = Vec::with_capacity(samples.len());
    let mut noise = Noise::new(encoding.dither());
    for (index, sample) in samples.iter().copied().enumerate() {
        if !sample.is_finite() {
            return Err(EncodeError::NotFinite {
                index,
                value: sample,
            });
        }
        let scaled = sample * format.scale();
        let step = noise.next();
        let shifted = scaled + step;
        let rounded = shifted.round();
        let highest = format.scale().round();
        if rounded > highest {
            return Err(EncodeError::OverRange {
                index,
                value: sample,
                format: format_name(format),
                highest: 1.0,
            });
        }
        // The negative side reaches one step further, because a signed integer
        // holds one more negative value than positive ones.
        let lowest = -(highest + 1.0);
        let value = if rounded < lowest { lowest } else { rounded };
        // `value` is a whole number that sits inside the integer range, so the
        // cast drops nothing.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "value is a rounded integer inside the format range"
        )]
        out.push(value as i32);
    }
    Ok(out)
}

/// Returns the name that an error message reports for a format.
fn format_name(format: crate::encode::Format) -> &'static str {
    match format {
        crate::encode::Format::WavF32 => "float WAV",
        crate::encode::Format::FlacI16 => "16-bit FLAC",
        crate::encode::Format::FlacI24 => "24-bit FLAC",
    }
}

/// The noise source of the dither policy.
///
/// The generator is a xorshift of a fixed seed, so every export gives the same
/// sequence. The state is small on purpose: the export runs one thread per
/// output, and a shared generator would make the bytes depend on the thread
/// order.
struct Noise {
    state: u64,
    active: bool,
}

impl Noise {
    /// The seed of the generator.
    ///
    /// Any non-zero value works, because the dither needs no cryptographic
    /// strength. The value stays fixed, so a repeat export gives the same file.
    const SEED: u64 = 0x2545_F491_4F6C_DD1D;

    /// Returns the generator for one policy.
    fn new(dither: Dither) -> Self {
        Self {
            state: Self::SEED,
            active: matches!(dither, Dither::Tpdf),
        }
    }

    /// Returns the next noise step.
    fn next(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }
        let first = self.uniform();
        let second = self.uniform();
        // The difference of two uniform values has a triangular density, which
        // is what TPDF names. Each value is in [0, 1), so the difference is in
        // (-1, 1).
        let triangular = first - second;
        triangular * 0.5
    }

    /// Returns the next uniform value in `[0, 1)`.
    fn uniform(&mut self) -> f32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        let top = self.state >> 40;
        // `top` holds 24 bits, which is the width of an f32 mantissa, so the
        // cast keeps every bit.
        #[expect(
            clippy::cast_precision_loss,
            reason = "top holds 24 bits, which an f32 mantissa holds exactly"
        )]
        let value = top as f32 / 16_777_216.0;
        value.min(1.0 - f32::EPSILON)
    }
}

#[cfg(test)]
mod tests {
    use super::Dither;
    use super::Noise;
    use crate::encode::Format;

    #[test]
    fn no_dither_gives_no_noise() {
        let mut noise = Noise::new(Dither::None);
        for _ in 0..16 {
            assert!(noise.next().abs() < f32::EPSILON);
        }
    }

    #[test]
    fn tpdf_stays_inside_its_bound() {
        let mut noise = Noise::new(Dither::Tpdf);
        for _ in 0..1_000 {
            let step = noise.next();
            assert!(step.abs() <= 0.5, "the step {step} is outside the bound");
        }
    }

    #[test]
    fn tpdf_is_reproducible() {
        let mut first = Noise::new(Dither::Tpdf);
        let mut second = Noise::new(Dither::Tpdf);
        for _ in 0..64 {
            assert_eq!(first.next(), second.next(), "the sequence must repeat");
        }
    }

    #[test]
    fn the_tolerance_grows_by_the_dither_width() {
        assert!(Dither::Tpdf.tolerance(Format::FlacI16) > Dither::None.tolerance(Format::FlacI16));
        assert_eq!(
            Dither::Tpdf.tolerance(Format::FlacI16),
            3.0 * Format::FlacI16.tolerance()
        );
    }
}
