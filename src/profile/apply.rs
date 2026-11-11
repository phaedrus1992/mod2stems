//! Applies a resolved profile to a live module.
//!
//! The profile is applied or it fails. A profile never degrades to another one
//! because a control is missing, as `docs/design.md` §2.1 requires.

use std::collections::HashSet;
use std::hash::BuildHasher;

use openmpt_ext::Module;

use crate::profile::ProfileConfig;
use crate::profile::ProfileError;

/// The ctl that turns the stock Amiga resampler on.
pub const AMIGA_CTL: &str = "render.resampler.emulate_amiga";
/// The ctl that selects the Amiga filter model.
pub const AMIGA_TYPE_CTL: &str = "render.resampler.emulate_amiga_type";

/// Applies the config to a module, or fails.
///
/// The call sets the controls that the profile names, in one order: the Amiga
/// model first, then the emulation switch, then the render params. The model
/// must be in place before the switch, because the library applies the model
/// only while emulation is already on.
///
/// # Errors
///
/// Returns an error when the module does not offer a control that the profile
/// needs, or when the library rejects a value. The module keeps the settings it
/// already had, so a failed call leaves no partial profile in place.
pub fn apply_profile(config: &ProfileConfig, module: &mut Module) -> Result<(), ProfileError> {
    if let Some(model) = config.amiga_model() {
        require_controls(module, &[AMIGA_CTL, AMIGA_TYPE_CTL])?;
        module
            .ctl_set_text(AMIGA_TYPE_CTL, model.ctl_value())
            .map_err(|error| library_error(&error))?;
        module
            .ctl_set_bool(AMIGA_CTL, true)
            .map_err(|error| library_error(&error))?;
    }
    for (param, value) in config.settings().render_params() {
        module
            .set_render_param(param, value)
            .map_err(|error| library_error(&error))?;
    }
    Ok(())
}

/// Returns the first control that the module does not offer.
///
/// The check runs before any setter, because the library accepts an unknown
/// key on some calls and reports a stored error only later. An explicit
/// presence check makes the failure name the missing control.
#[must_use]
pub fn missing_control<S: BuildHasher>(
    ctls: &HashSet<String, S>,
    wanted: &[&'static str],
) -> Option<&'static str> {
    wanted.iter().copied().find(|key| !ctls.contains(*key))
}

/// Fails with a named control when the module does not offer one.
fn require_controls(module: &Module, wanted: &[&'static str]) -> Result<(), ProfileError> {
    match missing_control(module.ctls(), wanted) {
        None => Ok(()),
        Some(key) => Err(ProfileError::MissingControl { key }),
    }
}

/// Wraps a library error with the operation that failed.
fn library_error(error: &openmpt_ext::OpenmptError) -> ProfileError {
    let operation = match error.op {
        "ctl set" => "the profile control set",
        "set render param" => "the profile render param set",
        _ => "the profile apply",
    };
    ProfileError::Library {
        operation,
        message: error.to_string(),
    }
}
