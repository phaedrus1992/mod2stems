//! The level arithmetic that the export measures before it encodes.
//!
//! The preflight sums the channel renders, measures the peak of every output,
//! and states the gain that fits the format. The sum is a level too, because a
//! sum of stems can exceed every stem that it holds (`docs/design.md` §5.1).

use crate::encode::EncodeError;
use crate::export::ExportError;

/// Returns the largest common gain that brings `peak` inside `highest`.
///
/// The caller reaches this call only when `peak` is above the range, so `peak`
/// is finite and positive.
#[must_use]
pub(crate) fn required_gain(highest: f32, peak: f32) -> f32 {
    if peak <= 0.0 {
        return highest;
    }
    highest / peak
}

/// Returns the larger of `current` and the peak of `samples`.
///
/// A sample that is not finite fails the check. `f32::max` ignores a NaN
/// operand, so the loop would otherwise read a broken render as a quiet one.
pub(crate) fn largest_peak(current: f32, samples: &[f32]) -> Result<f32, ExportError> {
    let mut peak = current;
    for (index, sample) in samples.iter().copied().enumerate() {
        if !sample.is_finite() {
            return Err(ExportError::from(EncodeError::NotFinite {
                index,
                value: sample,
            }));
        }
        peak = peak.max(sample.abs());
    }
    Ok(peak)
}

/// Adds `samples` into `sum`, and seeds `sum` from the first buffer.
pub(crate) fn add_into(sum: &mut Vec<f32>, samples: &[f32]) {
    if sum.is_empty() {
        *sum = samples.to_vec();
        return;
    }
    for (total, sample) in sum.iter_mut().zip(samples) {
        *total += sample;
    }
}

#[cfg(test)]
mod tests {
    use super::add_into;
    use super::largest_peak;
    use super::required_gain;
    use crate::encode::EncodeError;
    use crate::export::ExportError;
    use proptest::collection::vec;
    use proptest::prelude::any;
    use proptest::prelude::prop_assert;
    use proptest::prelude::prop_assert_eq;
    use proptest::proptest;
    use proptest::test_runner::Config;
    use proptest::test_runner::TestCaseError;

    proptest! {
        #![proptest_config(Config::with_cases(128))]

        /// A gain that answers an over-range peak never boosts, and it brings
        /// the peak inside the range.
        #[test]
        fn the_required_gain_attenuates_and_fits(
            highest in 0.01_f32..1.0,
            extra in 0.0_f32..8.0,
        ) {
            let peak = highest + extra.max(highest * 0.001);
            if peak <= highest {
                return Ok(());
            }
            let gain = required_gain(highest, peak);
            prop_assert!(gain > 0.0 && gain <= 1.0, "the gain {gain} is not an attenuation");
            let scaled = peak * gain;
            prop_assert!(
                scaled <= highest * (1.0 + f32::EPSILON),
                "the peak {peak} at {gain} reaches {scaled}, above {highest}"
            );
        }

        /// The peak of a finite set is the largest magnitude that it holds.
        #[test]
        fn the_peak_is_the_largest_magnitude(samples in vec(-8.0_f32..8.0, 0..64)) {
            let peak = largest_peak(0.0, &samples)
                .map_err(|error| TestCaseError::fail(error.to_string()))?;
            let expected = samples.iter().fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
            prop_assert_eq!(peak.to_bits(), expected.to_bits());
        }

        /// A non-finite sample fails, and the error names its position.
        #[test]
        fn a_non_finite_sample_names_its_index(
            before in 0_usize..16,
            after in 0_usize..16,
        ) {
            let mut samples = vec![0.5_f32; before];
            samples.push(f32::NAN);
            samples.extend(std::iter::repeat_n(0.5_f32, after));
            match largest_peak(0.0, &samples) {
                Err(ExportError::Encode(EncodeError::NotFinite { index, .. })) => {
                    prop_assert_eq!(index, before);
                }
                other => {
                    return Err(TestCaseError::fail(format!(
                        "a NaN at {before} gives {other:?}"
                    )));
                }
            }
        }

        /// The sum holds every sample of every buffer that it adds.
        #[test]
        fn the_sum_adds_each_sample(
            first in vec(-2.0_f32..2.0, 0..64),
            second in vec(-2.0_f32..2.0, 0..64),
        ) {
            let count = first.len().min(second.len());
            let mut sum: Vec<f32> = first.get(..count).unwrap_or_default().to_vec();
            add_into(&mut sum, second.get(..count).unwrap_or_default());
            prop_assert_eq!(sum.len(), count);
            for (index, total) in sum.iter().enumerate() {
                let (left, right) = (first[index], second[index]);
                prop_assert_eq!(total.to_bits(), (left + right).to_bits());
            }
        }

        /// A sum with no buffer keeps whatever it held.
        #[test]
        fn an_empty_add_changes_nothing(seed in vec(any::<f32>(), 0..32)) {
            let mut sum = seed.clone();
            add_into(&mut sum, &[]);
            prop_assert_eq!(sum, seed);
        }
    }
}
