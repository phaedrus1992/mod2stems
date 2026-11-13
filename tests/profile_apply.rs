//! Tests that a resolved profile reaches a live module, or fails.
//!
//! Most tests load vendored modules, so they exercise the library controls that
//! the design names. One test loads a module that `tests/support/it.rs` builds,
//! because the vendor corpus has no module that reads the random generator.
//! `docs/design.md` §2.1 requires that a requested profile is applied or fails,
//! and never degrades silently.

use std::collections::HashSet;

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

#[path = "support/fields.rs"]
mod fields;
#[path = "support/header.rs"]
mod header;
#[path = "support/it.rs"]
mod it;
#[path = "support/mt2.rs"]
mod mt2;

use fields::fixture_bytes;
use fields::load;
use fields::load_bytes;

/// Resolves a request that must resolve.
fn resolve(request: ProfileRequest) -> Result<ProfileConfig, String> {
    request.resolve().map_err(|error| error.to_string())
}

/// Loads a module from bytes, and keeps the library error type.
fn load_typed(bytes: &[u8]) -> Result<Module, ProfileError> {
    Module::from_bytes(bytes, &[])
        .map_err(|error| ProfileError::library(&error, "the fixture module load"))
}

/// Makes two plain renders of one module.
///
/// The Amiga check needs a deterministic baseline, so the caller compares these
/// two renders before it trusts a verdict. Each failure keeps its own type: a
/// render that hits the duration limit stays `TooLong`, not a library error.
fn two_plain_renders(bytes: &[u8]) -> Result<(Vec<f32>, Vec<f32>), ProfileError> {
    let mut first = load_typed(bytes)?;
    let plain = read_pass(&mut first, 48_000, None, DEFAULT_MAX_RENDER_SECONDS)?;
    let mut second = load_typed(bytes)?;
    let again = read_pass(&mut second, 48_000, None, DEFAULT_MAX_RENDER_SECONDS)?;
    Ok((plain, again))
}

/// Runs the whole Amiga check on one module, in the order the export uses.
fn verdict_for_bytes(bytes: &[u8]) -> Result<AmigaVerdict, ProfileError> {
    let (plain, again) = two_plain_renders(bytes)?;
    let mut module = load_typed(bytes)?;
    let emulated = read_pass(
        &mut module,
        48_000,
        Some(AmigaModel::A1200),
        DEFAULT_MAX_RENDER_SECONDS,
    )?;
    verify_amiga_applies(&plain, &again, &emulated)
}

/// Returns the comparison verdict for one fixture.
fn verdict_for(name: &str) -> Result<AmigaVerdict, String> {
    let bytes = fixture_bytes(name)?;
    verdict_for_bytes(&bytes).map_err(|error| error.to_string())
}

#[test]
fn clean_sets_the_windowed_sinc_length() -> Result<(), String> {
    let mut module = load("test.mod")?;
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
fn the_apply_path_refuses_clean_for_an_mt2_module() -> Result<(), String> {
    // `apply_profile` reads the type itself, so no caller can forget the
    // refusal that the design promises.
    let mut module = load_bytes(&mt2::nearest_resampling_mt2())?;
    let config = resolve(ProfileRequest::clean())?;
    match apply_profile(&config, &mut module) {
        Err(ProfileError::CleanDoesNotApply) => Ok(()),
        Err(error) => Err(format!(
            "the apply path reports {error:?}, expected CleanDoesNotApply"
        )),
        Ok(()) => Err("the apply path accepts an MT2 module for the clean profile".to_owned()),
    }
}

#[test]
fn amiga_enables_emulation_with_the_named_model() -> Result<(), String> {
    let mut module = load("test.mod")?;
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
    let mut module = load("test.mod")?;
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
    let mut module = load("test.mod")?;
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
        let mut module = load("test.mod")?;
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
fn a_rate_outside_the_renderer_range_fails_before_the_read() -> Result<(), String> {
    // The frame limit is computed from the rate. A bad rate must fail as a rate
    // error, not as a limit that the rate mapped to. The values sit one step
    // outside each end, so the check covers the boundary and not just far away.
    for rate in [0, 7_999, 192_001, 384_000] {
        let mut module = load("test.mod")?;
        match read_pass(&mut module, rate, None, DEFAULT_MAX_RENDER_SECONDS) {
            Err(ProfileError::RateOutOfRange {
                rate: found,
                lowest: 8000,
                highest: 192_000,
            }) if found == rate => {}
            other => {
                return Err(format!(
                    "the rate {rate} gives {other:?}, expected RateOutOfRange"
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn a_render_limit_stops_a_long_read() -> Result<(), String> {
    let mut module = load("test.mod")?;
    match read_pass(&mut module, 48_000, None, 0) {
        Ok(samples) => Err(format!(
            "the read returned {} samples at a zero second limit",
            samples.len()
        )),
        Err(ProfileError::TooLong { limit_seconds: 0 }) => Ok(()),
        Err(error) => Err(format!("the error is {error:?}, expected TooLong")),
    }
}

#[test]
fn a_limit_too_large_for_a_frame_count_fails() -> Result<(), String> {
    // The cap is a sample count. A limit whose count does not fit in a usize
    // would saturate to a count that never fires, so the guard must reject it
    // instead of widening the allocation bound.
    let mut module = load("test.mod")?;
    match read_pass(&mut module, 48_000, None, u64::MAX) {
        Err(ProfileError::LimitOutOfRange {
            limit_seconds,
            rate: 48_000,
        }) if limit_seconds == u64::MAX => Ok(()),
        other => Err(format!(
            "the limit u64::MAX gives {other:?}, expected LimitOutOfRange"
        )),
    }
}

#[test]
fn a_randomized_module_reports_nondeterministic() -> Result<(), String> {
    // The volume swing makes every triggered note read the process generator,
    // so two instances of the module render differently. The check must report
    // that condition instead of a verdict.
    let bytes = it::randomized_it();
    let (first, second) = two_plain_renders(&bytes).map_err(|error| error.to_string())?;
    match require_deterministic(&first, &second) {
        Err(ProfileError::Nondeterministic { .. }) => Ok(()),
        Err(error) => Err(format!(
            "the check reports {error:?}, expected Nondeterministic"
        )),
        Ok(()) => Err(format!(
            "two plain renders of the randomized fixture are identical, and both hold {} samples",
            first.len()
        )),
    }
}

#[test]
fn the_amiga_check_reports_the_random_render_as_an_error() -> Result<(), String> {
    let bytes = it::randomized_it();
    match verdict_for_bytes(&bytes) {
        Err(ProfileError::Nondeterministic { .. }) => Ok(()),
        Err(error) => Err(format!(
            "the check reports {error:?}, expected Nondeterministic"
        )),
        Ok(verdict) => Err(format!(
            "the check returns the verdict {verdict:?} for a randomized module"
        )),
    }
}

/// Renders one fixture with the named global interpolation filter length.
fn render_at_filter_length(name: &str, filter: i32) -> Result<Vec<f32>, String> {
    let bytes = fixture_bytes(name)?;
    let mut module = load_typed(&bytes).map_err(|error| error.to_string())?;
    module
        .set_render_param(RenderParam::InterpolationFilterLength, filter)
        .map_err(|error| error.to_string())?;
    read_pass(&mut module, 48_000, None, DEFAULT_MAX_RENDER_SECONDS)
        .map_err(|error| error.to_string())
}

#[test]
fn the_global_filter_length_does_not_reach_the_xm_or_mptm_render() -> Result<(), String> {
    // #25 records that a per-instrument `R...` mode wins over the global mode,
    // and that no C API reads it. A render comparison is the obvious fallback,
    // and this test pins why it does not work: the two formats that can carry
    // `R...` render the same samples at both filter lengths, so a comparison
    // has no signal to read. A library bump that changes this result makes the
    // test fail, which is the moment to revisit #25.
    for name in ["test.xm", "test.mptm"] {
        let sinc = render_at_filter_length(name, 8)?;
        let nearest = render_at_filter_length(name, 1)?;
        if sinc.len() != nearest.len() {
            return Err(format!(
                "{name} renders {} samples at length 8 and {} at length 1",
                sinc.len(),
                nearest.len()
            ));
        }
        if let Some(index) = sinc
            .iter()
            .zip(&nearest)
            .position(|(a, b)| a.to_bits() != b.to_bits())
        {
            return Err(format!(
                "{name} differs at sample {index}, so a render comparison can detect the global mode"
            ));
        }
    }
    Ok(())
}

#[test]
fn the_global_filter_length_reaches_the_mod_render() -> Result<(), String> {
    // The same comparison does have a signal on a MOD, so the test above is
    // not passing because the probe is broken.
    let sinc = render_at_filter_length("test.mod", 8)?;
    let nearest = render_at_filter_length("test.mod", 1)?;
    match sinc
        .iter()
        .zip(&nearest)
        .position(|(a, b)| a.to_bits() != b.to_bits())
    {
        Some(_) => Ok(()),
        None => Err("test.mod renders the same samples at both filter lengths".to_owned()),
    }
}
