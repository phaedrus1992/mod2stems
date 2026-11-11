//! Applies a resolved profile to a live module.
//!
//! The profile is applied or it fails. A profile never degrades to another one
//! because a control is missing, as `docs/design.md` §2.1 requires.

use openmpt_ext::Module;

use crate::profile::AmigaModel;
use crate::profile::ProfileConfig;
use crate::profile::ProfileError;

/// The ctl that turns the stock Amiga resampler on.
pub const AMIGA_CTL: &str = "render.resampler.emulate_amiga";
/// The ctl that selects the Amiga filter model.
pub const AMIGA_TYPE_CTL: &str = "render.resampler.emulate_amiga_type";

/// The controls that the `amiga` profile needs.
pub const AMIGA_CONTROLS: [&str; 2] = [AMIGA_CTL, AMIGA_TYPE_CTL];

/// Applies the config to a module, or fails.
///
/// The call sets the controls that the profile names, in one order: the Amiga
/// model first, then the emulation switch, then the render params.
///
/// The module must be a fresh instance. A failed call can leave the settings
/// that it already wrote, and it does not reset them. The export therefore
/// discards a module after a failed apply, and a partly applied profile never
/// renders.
///
/// # Errors
///
/// Returns an error when the module does not offer a control that the profile
/// needs, when the library rejects a value, or when the module reads back a
/// setting that the call did not set.
pub fn apply_profile(config: &ProfileConfig, module: &mut Module) -> Result<(), ProfileError> {
    if let Some(model) = config.amiga_model() {
        enable_amiga(module, model, "the profile control set")?;
    }
    for (param, value) in config.settings().render_params() {
        module
            .set_render_param(param, value)
            .map_err(|error| ProfileError::library(&error, "the profile render param set"))?;
        // The library snaps a filter length to a resampling mode, and reads the
        // mode back as the length of that mode. A different readback means the
        // profile did not take effect: `clean` needs windowed-sinc, and the
        // library answers 8 for it.
        let actual = module
            .render_param(param)
            .map_err(|error| ProfileError::library(&error, "the profile render param read"))?;
        if actual != value {
            return Err(ProfileError::SettingMismatch {
                setting: param_name(param),
                wanted: value,
                actual,
            });
        }
    }
    Ok(())
}

/// Turns Amiga emulation on with the named model, and reads both back.
///
/// The model goes in first. The library applies the model only while emulation
/// is already on, so the reverse order would leave the default model in place.
pub(crate) fn enable_amiga(
    module: &mut Module,
    model: AmigaModel,
    operation: &'static str,
) -> Result<(), ProfileError> {
    if let Some(key) = missing_control(module.ctls(), &AMIGA_CONTROLS) {
        return Err(ProfileError::MissingControl { key });
    }
    module
        .ctl_set_text(AMIGA_TYPE_CTL, model.as_str())
        .map_err(|error| ProfileError::library(&error, operation))?;
    module
        .ctl_set_bool(AMIGA_CTL, true)
        .map_err(|error| ProfileError::library(&error, operation))?;
    let found = module
        .ctl_get_text(AMIGA_TYPE_CTL)
        .map_err(|error| ProfileError::library(&error, operation))?;
    if found != model.as_str() {
        return Err(ProfileError::SettingTextMismatch {
            setting: AMIGA_TYPE_CTL,
            wanted: model.as_str(),
            actual: found,
        });
    }
    Ok(())
}

/// Returns the first control that the module does not offer.
///
/// The wrapper raises its own error for an unknown key, but that error reads
/// `Module::ctls` for the fix, which does not name the caller's intent. The
/// check states which control the profile needs and the build does not offer.
#[must_use]
pub fn missing_control<S: std::hash::BuildHasher>(
    ctls: &std::collections::HashSet<String, S>,
    wanted: &[&'static str],
) -> Option<&'static str> {
    wanted.iter().copied().find(|key| !ctls.contains(*key))
}

/// Returns the name that an error message uses for a render param.
fn param_name(param: openmpt_ext::RenderParam) -> &'static str {
    match param {
        openmpt_ext::RenderParam::InterpolationFilterLength => "interpolation filter length",
        openmpt_ext::RenderParam::MasterGainMillibel => "master gain",
        openmpt_ext::RenderParam::StereoSeparationPercent => "stereo separation",
        openmpt_ext::RenderParam::VolumeRampingStrength => "volume ramping",
    }
}
