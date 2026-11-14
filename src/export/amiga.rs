//! The Amiga applicability check that an `amiga` export runs first.
//!
//! The library accepts the Amiga switch on any module and reads it back, so the
//! switch proves nothing. The profile proves applicability by comparison: two
//! plain renders show that the module is deterministic, and the emulated
//! render must differ from them. Design: `docs/design.md` §2.1.

use crate::export::ExportError;
use crate::export::ExportPlan;
use crate::export::plan::load;
use crate::profile::read_pass;
use crate::profile::require_amiga_applies;
use crate::profile::verify_amiga_applies;
use crate::stems::StemError;

/// Fails when the module does not use the Amiga resampler.
///
/// `emulated` is the unmuted render of the export, which the profile rendered
/// with emulation on. The call renders the module twice more with emulation
/// off, and requires those two to match bit for bit. A randomized module gives
/// a mismatch, so the export reports that instead of a false verdict.
///
/// # Errors
///
/// Returns [`ExportError::Stem`] when a plain render fails, when the module is
/// randomized, or when the module does not use the resampler.
pub fn check(plan: &ExportPlan, emulated: &[f32]) -> Result<(), ExportError> {
    if !plan.config().amiga_enabled() {
        return Ok(());
    }
    let plain = plain_render(plan)?;
    let again = plain_render(plan)?;
    let verdict = verify_amiga_applies(&plain, &again, emulated)
        .map_err(|error| ExportError::Stem(StemError::from(error)))?;
    require_amiga_applies(verdict).map_err(|error| ExportError::Stem(StemError::from(error)))
}

/// Renders the module with Amiga emulation off.
fn plain_render(plan: &ExportPlan) -> Result<Vec<f32>, ExportError> {
    let mut module = load(plan.bytes())?;
    read_pass(&mut module, plan.rate(), None, plan.policy().max_seconds())
        .map_err(|error| ExportError::Stem(StemError::from(error)))
}
