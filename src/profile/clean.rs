//! The `clean` applicability check.
//!
//! `clean` sets the global interpolation filter length, and the library maps
//! that length onto the windowed-sinc resampler. A per-instrument resampling
//! mode wins over the global mode (`Sndmix.cpp:2564-2571`), and the MT2 drum
//! loader sets the nearest mode on a drum whose sample has the "no
//! interpolation" flag (`Load_mt2.cpp:1055-1064`, `1102-1104`).
//!
//! The C API exposes no way to read a per-instrument resampling mode: the
//! integer header has only `openmpt_module_get_num_instruments` and
//! `openmpt_module_get_instrument_name`. The profile therefore cannot name the
//! instruments that keep the nearest mode, and it refuses the module instead.
//! A refusal keeps the rule that `docs/design.md` §2.1 states: a requested
//! profile is applied, or it fails. It never degrades to another profile.

use crate::profile::ProfileError;

/// The module type that the `clean` profile cannot render as named.
///
/// The library sets this type from the MT2 loader, and returns it through the
/// `type` metadata key (`libopenmpt_impl.cpp:1247`).
const MT2_TYPE: &str = "mt2";

/// Fails when the module type names a format that `clean` cannot render.
///
/// The caller passes the value of the `type` metadata key
/// (see [`openmpt_ext::Module::metadata`]). A module that carries the MT2 type
/// fails, because a drum in that module can keep the nearest resampling mode
/// and ignore the windowed-sinc mode that `clean` sets.
///
/// The check reads the type, not the instruments, because the type is the only
/// fact that the C API exposes about a per-instrument resampling decision.
///
/// # Errors
///
/// Returns [`ProfileError::CleanDoesNotApply`] when `module_type` names the MT2
/// format.
pub fn require_clean_applies(module_type: &str) -> Result<(), ProfileError> {
    if module_type.eq_ignore_ascii_case(MT2_TYPE) {
        return Err(ProfileError::CleanDoesNotApply {
            module_type: module_type.to_owned(),
        });
    }
    Ok(())
}
