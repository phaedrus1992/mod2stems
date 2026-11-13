//! The Amiga applicability check.
//!
//! libopenmpt accepts `render.resampler.emulate_amiga = true` on any module,
//! and reads it back as `true`. The readback reports the setting, not whether
//! the Amiga resampler is used. The library applies the resampler only when an
//! internal flag is set, and no C API exposes that flag.
//!
//! The profile therefore proves applicability by comparison: render the module
//! with emulation off and on, and require the two renders to differ.
//! Design: `docs/design/pre-implementation-validation.md` §6.
//!
//! The comparison needs a baseline first. The library seeds the random state of
//! each instance from a process-wide generator, and no C API sets the seed, so
//! a randomized module renders differently on every instance
//! (`docs/design/pre-implementation-validation.md` §1). Two plain renders detect
//! that condition, and the check reports it instead of a verdict.

use crate::profile::ProfileError;

/// What the comparison of two renders shows about Amiga emulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmigaVerdict {
    /// The renders differ, so the module uses the Amiga resampler.
    Applies,
    /// The renders are identical, so the profile does not apply to the module.
    DoesNotApply,
}

/// Compares two plain renders and an emulated render of the same module.
///
/// `plain` is the first render, made with Amiga emulation off, `again` is a
/// second render with the same settings, and `emulated` is the render with
/// emulation on. All three start from the beginning of the same song, with the
/// same rate and the same controls.
///
/// The call runs the determinism check on the two plain renders itself. A
/// randomized module renders differently on every instance, so its emulated
/// render would compare unequal for a reason that has nothing to do with the
/// Amiga resampler. The check stops that case, and the caller cannot skip it.
///
/// The comparison is exact, by sample bits. A module that does not use the
/// Amiga resampler takes the same code path in both renders, so the samples
/// are bit-identical. Value equality would report a NaN pair as a difference
/// and a `0.0`/`-0.0` pair as a match, and neither is a reason to decide a
/// profile.
///
/// # Errors
///
/// Returns [`ProfileError::NondeterministicLength`] or
/// [`ProfileError::Nondeterministic`] when the two plain renders differ, and
/// [`ProfileError::AmigaComparisonLength`] when a plain render and the emulated
/// render differ in length.
pub fn verify_amiga_applies(
    plain: &[f32],
    again: &[f32],
    emulated: &[f32],
) -> Result<AmigaVerdict, ProfileError> {
    require_deterministic(plain, again)?;
    if plain.len() != emulated.len() {
        return Err(ProfileError::AmigaComparisonLength {
            plain: plain.len(),
            emulated: emulated.len(),
        });
    }
    if same_samples(plain, emulated) {
        return Ok(AmigaVerdict::DoesNotApply);
    }
    Ok(AmigaVerdict::Applies)
}

/// Fails when the module does not use the Amiga resampler.
///
/// The export calls this after [`verify_amiga_applies`], so an `amiga` profile
/// that the module cannot honor fails instead of rendering as plain audio.
///
/// # Errors
///
/// Returns [`ProfileError::AmigaDoesNotApply`] when the verdict is
/// [`AmigaVerdict::DoesNotApply`].
pub fn require_amiga_applies(verdict: AmigaVerdict) -> Result<(), ProfileError> {
    match verdict {
        AmigaVerdict::Applies => Ok(()),
        AmigaVerdict::DoesNotApply => Err(ProfileError::AmigaDoesNotApply),
    }
}

/// Reports whether two renders hold the same samples, bit for bit.
fn same_samples(left: &[f32], right: &[f32]) -> bool {
    left.iter()
        .zip(right)
        .all(|(a, b)| a.to_bits() == b.to_bits())
}

/// Returns the index of the first sample that two renders do not share.
///
/// The baseline render must match the second plain render, or the module reads
/// random state.
///
/// # Errors
///
/// Returns [`ProfileError::NondeterministicLength`] with both lengths when the
/// two renders differ in length, and [`ProfileError::Nondeterministic`] with
/// the index of the first difference when their samples differ. The caller
/// reports that error, and it does not treat the module as Amiga or as
/// deterministic.
pub fn require_deterministic(first: &[f32], second: &[f32]) -> Result<(), ProfileError> {
    if first.len() != second.len() {
        return Err(ProfileError::NondeterministicLength {
            first: first.len(),
            second: second.len(),
        });
    }
    match first
        .iter()
        .zip(second)
        .position(|(a, b)| a.to_bits() != b.to_bits())
    {
        None => Ok(()),
        Some(sample) => Err(ProfileError::Nondeterministic { sample }),
    }
}
