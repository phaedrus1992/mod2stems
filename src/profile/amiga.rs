//! The Amiga applicability check.
//!
//! libopenmpt accepts `render.resampler.emulate_amiga = true` on any module,
//! and reads it back as `true`. The readback reports the setting, not whether
//! the Amiga resampler is used. The library applies the resampler only when an
//! internal flag is set, and no C API exposes that flag.
//!
//! The profile therefore proves applicability by comparison: render the module
//! with emulation off and on, and require the two renders to differ.
//! The "off" side is the unmuted trace render that the pipeline already makes,
//! so the check costs one extra single-track render.
//! Design: `docs/design/pre-implementation-validation.md` §6.

use openmpt_ext::Module;

use crate::profile::AmigaModel;
use crate::profile::ProfileError;
use crate::profile::apply::AMIGA_CTL;
use crate::profile::apply::AMIGA_TYPE_CTL;
use crate::profile::apply::missing_control;

/// The frames that one detection read requests.
///
/// A larger block lowers the call count. A smaller block lets the song end
/// land closer to the true frame count, which keeps the two passes equal when
/// the song has a fractional block.
const DETECTION_BLOCK_FRAMES: usize = 4800;

/// What the comparison of two renders shows about Amiga emulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmigaVerdict {
    /// The renders differ, so the module uses the Amiga resampler.
    Applies,
    /// The renders are identical, so the profile does not apply to the module.
    DoesNotApply,
}

/// Compares a plain render with an emulated render of the same module.
///
/// `plain` is the render with Amiga emulation off, and `emulated` is the render
/// with it on. Both renders start from the beginning of the same song, with the
/// same rate and the same controls, so a difference in the samples is the only
/// thing that can separate them.
///
/// The comparison is exact. A module that does not use the Amiga resampler
/// takes the same code path in both renders, so the samples are bit-identical.
///
/// # Errors
///
/// Returns an error when the two renders differ in length, because a length
/// difference means the two renders did not use the same settings. It is not
/// evidence that the profile applies.
pub fn verify_amiga_applies(plain: &[f32], emulated: &[f32]) -> Result<AmigaVerdict, ProfileError> {
    if plain.len() != emulated.len() {
        return Err(ProfileError::AmigaComparisonLength {
            plain: plain.len(),
            emulated: emulated.len(),
        });
    }
    if plain == emulated {
        return Ok(AmigaVerdict::DoesNotApply);
    }
    Ok(AmigaVerdict::Applies)
}

/// Reads one whole song with the given Amiga emulation setting.
///
/// `model` is `None` for a plain pass, and `Some(model)` for an emulated pass.
/// The module must be at the start of the song, which it is after a load. The
/// read stops at the song end.
///
/// # Errors
///
/// Returns an error when an emulated pass meets a module that lacks a control
/// that the pass needs, or when a library call fails.
pub fn read_pass(
    module: &mut Module,
    rate: i32,
    model: Option<AmigaModel>,
) -> Result<Vec<f32>, ProfileError> {
    match model {
        Some(model) => enable_emulation(module, model)?,
        None => module
            .ctl_set_bool(AMIGA_CTL, false)
            .map_err(|error| library_error(&error))?,
    }
    let mut out = Vec::new();
    let mut buf = vec![0.0_f32; DETECTION_BLOCK_FRAMES * 2];
    loop {
        let frames = module
            .read_float(rate, &mut buf)
            .map_err(|error| library_error(&error))?;
        if frames == 0 {
            break;
        }
        // The library writes only `frames` frames. The tail of the buffer
        // still holds the previous block, so a copy of the whole buffer would
        // compare stale samples.
        let written = frames.saturating_mul(2);
        out.extend_from_slice(&buf[..written]);
    }
    Ok(out)
}

/// Turns Amiga emulation on with the named model.
///
/// The model goes in first. The library applies the model only while emulation
/// is already on, so the reverse order would leave the default model in place.
fn enable_emulation(module: &mut Module, model: AmigaModel) -> Result<(), ProfileError> {
    if let Some(key) = missing_control(module.ctls(), &[AMIGA_CTL, AMIGA_TYPE_CTL]) {
        return Err(ProfileError::MissingControl { key });
    }
    module
        .ctl_set_text(AMIGA_TYPE_CTL, model.ctl_value())
        .map_err(|error| library_error(&error))?;
    module
        .ctl_set_bool(AMIGA_CTL, true)
        .map_err(|error| library_error(&error))
}

/// Wraps a library error with the operation that failed.
fn library_error(error: &openmpt_ext::OpenmptError) -> ProfileError {
    ProfileError::Library {
        operation: "the Amiga applicability check",
        message: error.to_string(),
    }
}
