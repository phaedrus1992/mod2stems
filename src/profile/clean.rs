//! The `clean` applicability check.
//!
//! `clean` sets the global interpolation filter length, but a per-instrument
//! resampling mode wins over it (`Sndmix.cpp:2565-2571`). The C API exposes no
//! per-instrument resampling read, so the profile refuses a module that it
//! cannot render as named. The module type covers the MT2 loader only, so the
//! `R...` property stays open in #25.
//! Design: `docs/design/pre-implementation-validation.md` §6.

use crate::profile::ProfileError;

/// The module type that the `clean` profile cannot render as named.
///
/// The MT2 loader returns this type through the `type` metadata key
/// (`Load_mt2.cpp:453`, `libopenmpt_impl.cpp:1247`).
const MT2_TYPE: &str = "mt2";

/// Fails when the module type names a format that `clean` cannot render.
///
/// The caller passes the value of the `type` metadata key, read through
/// [`openmpt_ext::Module::metadata`]. A missing key returns an empty string,
/// so an empty value fails here rather than passing as an unknown format.
///
/// # Errors
///
/// Returns [`ProfileError::MissingModuleType`] for an empty value, because a
/// module that reports no type cannot be shown to keep the profile promise.
/// Returns [`ProfileError::CleanDoesNotApply`] when the value names the MT2
/// format.
pub fn require_clean_applies(module_type: &str) -> Result<(), ProfileError> {
    // One normalization for both checks, so a padded token cannot pass the
    // empty check and then miss the refusal.
    let token = module_type.trim();
    if token.is_empty() {
        return Err(ProfileError::MissingModuleType);
    }
    if token.eq_ignore_ascii_case(MT2_TYPE) {
        return Err(ProfileError::CleanDoesNotApply);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::require_clean_applies;
    use crate::profile::ProfileError;

    #[test]
    fn the_mt2_type_is_refused_in_every_spelling() {
        // A drum in an MT2 module can keep the nearest resampling mode, and the
        // C API exposes no way to name the drum. The padded tokens prove that
        // one normalization feeds both checks.
        for module_type in ["mt2", "MT2", " mt2 "] {
            assert_eq!(
                require_clean_applies(module_type),
                Err(ProfileError::CleanDoesNotApply),
                "the type {module_type:?} passed the clean check"
            );
        }
    }

    #[test]
    fn a_missing_type_is_refused() {
        // `Module::metadata` returns an empty string for a missing key. An
        // empty type cannot keep the profile promise.
        for module_type in ["", "  "] {
            assert_eq!(
                require_clean_applies(module_type),
                Err(ProfileError::MissingModuleType),
                "the type {module_type:?} passed the clean check"
            );
        }
    }

    #[test]
    fn every_other_type_passes() {
        for module_type in ["mod", "s3m", "xm", "it", "mptm"] {
            assert_eq!(
                require_clean_applies(module_type),
                Ok(()),
                "the type {module_type:?} failed the clean check"
            );
        }
    }
}
