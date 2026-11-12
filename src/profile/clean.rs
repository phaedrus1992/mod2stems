//! The `clean` applicability check.
//!
//! `clean` sets the global interpolation filter length, but a per-instrument
//! resampling mode wins over it (`Sndmix.cpp:2565-2571`). The C API exposes no
//! per-instrument resampling read, so the profile cannot name the affected
//! instruments and refuses the module instead.
//!
//! The check reads the module type, so it covers the MT2 loader only. An
//! extended instrument property `R...` sets the same field with no type
//! change, and it stays undetected until #25 lands.
//!
//! A render comparison cannot close that gap. The global filter length changes
//! the render of `test.mod` and `test.s3m`, and leaves `test.xm` and `test.mptm`
//! bit-identical, so the comparison has no signal for the formats that carry
//! `R...`. Design: `docs/design/pre-implementation-validation.md` §6.

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
    if module_type.trim().is_empty() {
        return Err(ProfileError::MissingModuleType);
    }
    if module_type.eq_ignore_ascii_case(MT2_TYPE) {
        return Err(ProfileError::CleanDoesNotApply);
    }
    Ok(())
}
