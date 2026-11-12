//! Tests that a resolved profile reaches a live module, or fails.
//!
//! The tests load vendored modules, so they exercise the library controls that
//! the design names. `docs/design.md` §2.1 requires that a requested profile is
//! applied or fails, and never degrades silently.

use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;

use mod2stems::AmigaModel;
use mod2stems::DEFAULT_MAX_RENDER_SECONDS;
use mod2stems::Profile;
use mod2stems::ProfileConfig;
use mod2stems::ProfileError;
use mod2stems::ProfileRequest;
use mod2stems::profile::AmigaVerdict;
use mod2stems::profile::apply::AMIGA_CONTROLS;
use mod2stems::profile::apply::AMIGA_CTL;
use mod2stems::profile::apply::AMIGA_TYPE_CTL;
use mod2stems::profile::apply::apply_profile;
use mod2stems::profile::apply::missing_control;
use mod2stems::profile::read_pass;
use mod2stems::profile::require_deterministic;
use mod2stems::profile::verify_amiga_applies;
use openmpt_ext::Module;
use openmpt_ext::RenderParam;

/// Returns the path of a vendored test module.
fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("crates/openmpt-ext-sys/vendor/openmpt/test")
        .join(name)
}

/// Loads a vendored test module.
fn load_named(name: &str) -> Result<Module, String> {
    let path = fixture_path(name);
    let bytes =
        std::fs::read(&path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    Module::from_bytes(&bytes, &[]).map_err(|error| error.to_string())
}

/// Loads the vendored `test.mod`.
fn load() -> Result<Module, String> {
    load_named("test.mod")
}

/// Resolves a request that must resolve.
fn resolve(request: ProfileRequest) -> Result<ProfileConfig, String> {
    request.resolve().map_err(|error| error.to_string())
}

/// Returns the comparison verdict for one fixture.
///
/// The check needs a deterministic baseline, so the call makes two plain
/// renders and reports a module that fails that test.
fn verdict_for(name: &str) -> Result<AmigaVerdict, String> {
    let first = {
        let mut module = load_named(name)?;
        read_pass(&mut module, 48_000, None, DEFAULT_MAX_RENDER_SECONDS)
            .map_err(|error| error.to_string())?
    };
    let second = {
        let mut module = load_named(name)?;
        read_pass(&mut module, 48_000, None, DEFAULT_MAX_RENDER_SECONDS)
            .map_err(|error| error.to_string())?
    };
    require_deterministic(&first, &second).map_err(|error| error.to_string())?;
    let emulated = {
        let mut module = load_named(name)?;
        read_pass(
            &mut module,
            48_000,
            Some(AmigaModel::A1200),
            DEFAULT_MAX_RENDER_SECONDS,
        )
        .map_err(|error| error.to_string())?
    };
    verify_amiga_applies(&first, &emulated).map_err(|error| error.to_string())
}

#[test]
fn clean_sets_the_windowed_sinc_length() -> Result<(), String> {
    let mut module = load()?;
    let config = resolve(ProfileRequest::clean())?;
    apply_profile(&config, &mut module).map_err(|error| error.to_string())?;
    let length = module
        .render_param(RenderParam::InterpolationFilterLength)
        .map_err(|error| error.to_string())?;
    if length != 8 {
        return Err(format!("the filter length is {length}, expected 8"));
    }
    Ok(())
}

#[test]
fn amiga_enables_emulation_with_the_named_model() -> Result<(), String> {
    let mut module = load()?;
    let config = resolve(ProfileRequest::amiga(Some(AmigaModel::A500)))?;
    apply_profile(&config, &mut module).map_err(|error| error.to_string())?;
    let enabled = module
        .ctl_get_bool(AMIGA_CTL)
        .map_err(|error| error.to_string())?;
    if !enabled {
        return Err("the amiga ctl reads false after the profile applied".to_owned());
    }
    let model = module
        .ctl_get_text(AMIGA_TYPE_CTL)
        .map_err(|error| error.to_string())?;
    if model != "a500" {
        return Err(format!(
            "the amiga model reads {model:?}, expected \"a500\""
        ));
    }
    Ok(())
}

#[test]
fn the_model_is_set_before_emulation_turns_on() -> Result<(), String> {
    // The library applies the model only while emulation is already on. This
    // test sets the model on a module whose emulation is off, as the apply call
    // does, and reads the model back after the switch. A wrong order would
    // leave the library default model in place.
    let mut module = load()?;
    module
        .ctl_set_text(AMIGA_TYPE_CTL, "unfiltered")
        .map_err(|error| error.to_string())?;
    module
        .ctl_set_bool(AMIGA_CTL, true)
        .map_err(|error| error.to_string())?;
    let model = module
        .ctl_get_text(AMIGA_TYPE_CTL)
        .map_err(|error| error.to_string())?;
    if model != "unfiltered" {
        return Err(format!(
            "the amiga model reads {model:?}, expected \"unfiltered\""
        ));
    }
    Ok(())
}

#[test]
fn compatible_leaves_amiga_emulation_off() -> Result<(), String> {
    let mut module = load()?;
    let config = resolve(ProfileRequest::default())?;
    apply_profile(&config, &mut module).map_err(|error| error.to_string())?;
    let enabled = module
        .ctl_get_bool(AMIGA_CTL)
        .map_err(|error| error.to_string())?;
    if enabled {
        return Err("the compatible profile turned amiga emulation on".to_owned());
    }
    Ok(())
}

#[test]
fn every_profile_resolves_for_the_module() -> Result<(), String> {
    let requests = [
        ProfileRequest::default(),
        ProfileRequest::clean(),
        ProfileRequest::amiga(Some(AmigaModel::A1200)),
    ];
    for request in requests {
        let profile = request.profile();
        let config = resolve(request)?;
        let mut module = load()?;
        let missing = apply_profile(&config, &mut module)
            .err()
            .map(|error| error.to_string());
        if let Some(missing) = missing {
            return Err(format!("the {profile:?} profile failed: {missing}"));
        }
    }
    Ok(())
}

#[test]
fn a_missing_control_is_named() -> Result<(), String> {
    let ctls = HashSet::from([AMIGA_CTL.to_owned()]);
    if missing_control(&ctls, &AMIGA_CONTROLS) != Some(AMIGA_TYPE_CTL) {
        return Err(format!(
            "the check reports {:?}, expected {AMIGA_TYPE_CTL:?}",
            missing_control(&ctls, &AMIGA_CONTROLS)
        ));
    }
    if missing_control(&ctls, &[AMIGA_CTL]).is_some() {
        return Err("the check reports a missing control that the set holds".to_owned());
    }
    Ok(())
}

#[test]
fn the_profile_applies_to_an_amiga_module() -> Result<(), String> {
    let verdict = verdict_for("test.mod")?;
    if verdict != AmigaVerdict::Applies {
        return Err(format!(
            "test.mod reports {verdict:?}, and the fixture stays inside the Amiga note range"
        ));
    }
    Ok(())
}

#[test]
fn the_profile_does_not_apply_to_a_non_amiga_module() -> Result<(), String> {
    let verdict = verdict_for("test.xm")?;
    if verdict != AmigaVerdict::DoesNotApply {
        return Err(format!(
            "test.xm reports {verdict:?}, and an XM module has no Amiga resampler"
        ));
    }
    Ok(())
}

#[test]
fn one_rate_reaches_every_render() -> Result<(), String> {
    let config = resolve(ProfileRequest::amiga(Some(AmigaModel::A1200)).with_rate(44_100))?;
    if config.profile() != Profile::Amiga {
        return Err(format!("the config profile is {:?}", config.profile()));
    }
    if config.rate() != 44_100 {
        return Err(format!("the config rate is {}", config.rate()));
    }
    Ok(())
}

#[test]
fn a_render_limit_stops_a_long_read() -> Result<(), String> {
    let mut module = load()?;
    match read_pass(&mut module, 48_000, None, 0) {
        Ok(samples) => Err(format!(
            "the read returned {} samples at a zero second limit",
            samples.len()
        )),
        Err(ProfileError::TooLong { limit_seconds: 0 }) => Ok(()),
        Err(error) => Err(format!("the error is {error:?}, expected TooLong")),
    }
}
