//! Tests the rendering profiles and the output rate resolution.
//!
//! The checks cover the resolution rules that `docs/design.md` §2 states, and
//! the Amiga applicability rule from `docs/design/pre-implementation-validation.md`
//! §1 and §6. `tests/profile_properties.rs` holds the property tests.

use mod2stems::AmigaModel;
use mod2stems::DEFAULT_OUTPUT_RATE;
use mod2stems::Profile;
use mod2stems::ProfileConfig;
use mod2stems::ProfileError;
use mod2stems::ProfileRequest;
use mod2stems::profile::AmigaVerdict;
use mod2stems::profile::require_amiga_applies;
use mod2stems::profile::require_deterministic;
use mod2stems::profile::verify_amiga_applies;

/// Resolves one request that must succeed.
fn resolve(request: ProfileRequest) -> Result<ProfileConfig, String> {
    request.resolve().map_err(|error| error.to_string())
}

/// Resolves one request that must fail.
fn resolve_err(request: ProfileRequest) -> Result<ProfileError, String> {
    match request.resolve() {
        Ok(config) => Err(format!(
            "the request must fail, but it resolved to {config:?}"
        )),
        Err(error) => Ok(error),
    }
}

/// Compares two renders that must compare.
fn compare(plain: &[f32], emulated: &[f32]) -> Result<AmigaVerdict, String> {
    verify_amiga_applies(plain, emulated).map_err(|error| error.to_string())
}

#[test]
fn default_profile_is_compatible() -> Result<(), String> {
    if Profile::default() != Profile::Compatible {
        return Err(format!("the default profile is {:?}", Profile::default()));
    }
    if ProfileRequest::default().profile() != Profile::Compatible {
        return Err("a default request does not name the compatible profile".to_owned());
    }
    Ok(())
}

#[test]
fn default_rate_is_the_library_recommendation() -> Result<(), String> {
    if DEFAULT_OUTPUT_RATE != 48_000 {
        return Err(format!(
            "the default rate is {DEFAULT_OUTPUT_RATE}, expected 48000"
        ));
    }
    if ProfileRequest::default().rate() != 48_000 {
        return Err(format!(
            "a default request gives {} Hz, expected 48000 Hz",
            ProfileRequest::default().rate()
        ));
    }
    Ok(())
}

#[test]
fn compatible_records_the_resolved_settings() -> Result<(), String> {
    let config = resolve(ProfileRequest::default())?;
    if config.profile() != Profile::Compatible {
        return Err(format!("the config profile is {:?}", config.profile()));
    }
    if config.rate() != DEFAULT_OUTPUT_RATE {
        return Err(format!(
            "the config rate is {}, expected {DEFAULT_OUTPUT_RATE}",
            config.rate()
        ));
    }
    if config.amiga_model().is_some() {
        return Err("the compatible profile names an Amiga model".to_owned());
    }
    if config.amiga_enabled() {
        return Err("the compatible profile turns Amiga emulation on".to_owned());
    }
    if config.settings().interpolation_filter_length().is_some() {
        return Err("the compatible profile sets an interpolation filter length".to_owned());
    }
    if !config.settings().render_params().is_empty() {
        return Err(format!(
            "the compatible profile sets {} render params, expected none",
            config.settings().render_params().len()
        ));
    }
    Ok(())
}

#[test]
fn clean_selects_windowed_sinc() -> Result<(), String> {
    let config = resolve(ProfileRequest::clean())?;
    if config.profile() != Profile::Clean {
        return Err(format!("the config profile is {:?}", config.profile()));
    }
    if config.settings().interpolation_filter_length() != Some(8) {
        return Err(format!(
            "the filter length is {:?}, expected Some(8)",
            config.settings().interpolation_filter_length()
        ));
    }
    if config.amiga_enabled() {
        return Err("the clean profile turns Amiga emulation on".to_owned());
    }
    if config.settings().render_params().len() != 1 {
        return Err(format!(
            "the clean profile sets {} render params, expected 1",
            config.settings().render_params().len()
        ));
    }
    Ok(())
}

#[test]
fn amiga_requires_a_model() -> Result<(), String> {
    let error = resolve_err(ProfileRequest::amiga(None))?;
    if error != ProfileError::MissingAmigaModel {
        return Err(format!(
            "the error is {error:?}, expected MissingAmigaModel"
        ));
    }
    if !error.to_string().contains("a500") {
        return Err(format!("the message {error} names no model to choose"));
    }
    Ok(())
}

#[test]
fn amiga_with_a_model_enables_emulation() -> Result<(), String> {
    let config = resolve(ProfileRequest::amiga(Some(AmigaModel::A500)))?;
    if !config.amiga_enabled() {
        return Err("the amiga profile leaves emulation off".to_owned());
    }
    if config.amiga_model() != Some(AmigaModel::A500) {
        return Err(format!(
            "the model is {:?}, expected Some(A500)",
            config.amiga_model()
        ));
    }
    if config.settings().interpolation_filter_length().is_some() {
        return Err("the amiga profile sets an interpolation filter length".to_owned());
    }
    Ok(())
}

#[test]
fn compatible_rejects_an_amiga_model() -> Result<(), String> {
    let error = resolve_err(ProfileRequest::default().with_amiga_model(AmigaModel::A500))?;
    if !matches!(error, ProfileError::UnexpectedAmigaModel { .. }) {
        return Err(format!(
            "the error is {error:?}, expected UnexpectedAmigaModel"
        ));
    }
    Ok(())
}

#[test]
fn clean_rejects_an_amiga_model() -> Result<(), String> {
    let error = resolve_err(ProfileRequest::clean().with_amiga_model(AmigaModel::A1200))?;
    if !matches!(error, ProfileError::UnexpectedAmigaModel { .. }) {
        return Err(format!(
            "the error is {error:?}, expected UnexpectedAmigaModel"
        ));
    }
    Ok(())
}

#[test]
fn a_rate_below_the_renderer_range_fails() -> Result<(), String> {
    let error = resolve_err(ProfileRequest::default().with_rate(4_000))?;
    let wanted = ProfileError::RateOutOfRange {
        rate: 4_000,
        lowest: 8_000,
        highest: 192_000,
    };
    if error != wanted {
        return Err(format!("the error is {error:?}, expected {wanted:?}"));
    }
    Ok(())
}

#[test]
fn a_rate_above_the_renderer_range_fails() -> Result<(), String> {
    let error = resolve_err(ProfileRequest::default().with_rate(384_000))?;
    if !matches!(error, ProfileError::RateOutOfRange { .. }) {
        return Err(format!("the error is {error:?}, expected RateOutOfRange"));
    }
    Ok(())
}

#[test]
fn the_range_ends_are_inclusive() -> Result<(), String> {
    for rate in [8_000, 192_000] {
        let config = resolve(ProfileRequest::default().with_rate(rate))?;
        if config.rate() != rate {
            return Err(format!(
                "the config rate is {}, expected {rate}",
                config.rate()
            ));
        }
    }
    Ok(())
}

#[test]
fn an_explicit_rate_reaches_the_config() -> Result<(), String> {
    let config = resolve(ProfileRequest::default().with_rate(44_100))?;
    if config.rate() != 44_100 {
        return Err(format!(
            "the config rate is {}, expected 44100",
            config.rate()
        ));
    }
    Ok(())
}

#[test]
fn profile_names_round_trip() -> Result<(), String> {
    for profile in [Profile::Compatible, Profile::Amiga, Profile::Clean] {
        let parsed = Profile::parse(profile.as_str()).map_err(|error| error.to_string())?;
        if parsed != profile {
            return Err(format!("{:?} parses to {parsed:?}", profile.as_str()));
        }
    }
    Ok(())
}

#[test]
fn an_unknown_profile_name_keeps_the_text() -> Result<(), String> {
    match Profile::parse("hardware") {
        Ok(profile) => Err(format!("the name parsed to {profile:?}")),
        Err(ProfileError::UnknownName { name }) if name == "hardware" => Ok(()),
        Err(error) => Err(format!("the error is {error:?}, expected UnknownName")),
    }
}

#[test]
fn differing_renders_mean_the_profile_applies() -> Result<(), String> {
    let plain = [0.0_f32, 0.5, 0.25, 0.125];
    let emulated = [0.0_f32, 0.5, 0.75, 0.125];
    let verdict = compare(&plain, &emulated)?;
    if verdict != AmigaVerdict::Applies {
        return Err(format!("the verdict is {verdict:?}, expected Applies"));
    }
    Ok(())
}

#[test]
fn identical_renders_mean_the_profile_does_not_apply() -> Result<(), String> {
    let plain = [0.0_f32, 0.5, 0.25];
    let verdict = compare(&plain, &plain)?;
    if verdict != AmigaVerdict::DoesNotApply {
        return Err(format!("the verdict is {verdict:?}, expected DoesNotApply"));
    }
    Ok(())
}

#[test]
fn a_verdict_that_does_not_apply_becomes_the_profile_error() -> Result<(), String> {
    if require_amiga_applies(AmigaVerdict::Applies).is_err() {
        return Err("the Applies verdict reported an error".to_owned());
    }
    match require_amiga_applies(AmigaVerdict::DoesNotApply) {
        Ok(()) => Err("the DoesNotApply verdict reported a success".to_owned()),
        Err(ProfileError::AmigaDoesNotApply) => Ok(()),
        Err(error) => Err(format!(
            "the error is {error:?}, expected AmigaDoesNotApply"
        )),
    }
}

#[test]
fn renders_of_different_lengths_are_not_a_comparison() -> Result<(), String> {
    let error = match verify_amiga_applies(&[0.0_f32, 1.0], &[0.0_f32]) {
        Ok(verdict) => return Err(format!("the check reported {verdict:?}")),
        Err(error) => error,
    };
    if !matches!(error, ProfileError::AmigaComparisonLength { .. }) {
        return Err(format!(
            "the error is {error:?}, expected AmigaComparisonLength"
        ));
    }
    Ok(())
}

#[test]
fn a_single_differing_sample_is_enough() -> Result<(), String> {
    let mut emulated = vec![0.25_f32; 65_536];
    emulated[65_535] = 0.5;
    let plain = vec![0.25_f32; 65_536];
    let verdict = compare(&plain, &emulated)?;
    if verdict != AmigaVerdict::Applies {
        return Err(format!("the verdict is {verdict:?}, expected Applies"));
    }
    Ok(())
}

#[test]
fn a_nan_pair_counts_as_identical() -> Result<(), String> {
    // Value equality reports a NaN as different from itself. Two renders that
    // hold the same NaN pattern took the same path, so the bits must decide.
    let warm = [f32::NAN, 0.5];
    let cool = [f32::NAN, 0.5];
    let verdict = compare(&warm, &cool)?;
    if verdict != AmigaVerdict::DoesNotApply {
        return Err(format!("the verdict is {verdict:?}, expected DoesNotApply"));
    }
    Ok(())
}

#[test]
fn a_signed_zero_pair_counts_as_different() -> Result<(), String> {
    // `0.0` and `-0.0` compare equal as values, but the mixer wrote different
    // bits, so the two renders took different paths.
    let positive = [0.0_f32, 0.5];
    let negative = [-0.0_f32, 0.5];
    let verdict = compare(&positive, &negative)?;
    if verdict != AmigaVerdict::Applies {
        return Err(format!("the verdict is {verdict:?}, expected Applies"));
    }
    Ok(())
}

#[test]
fn two_plain_renders_that_differ_are_reported_as_random() -> Result<(), String> {
    let first = [0.0_f32, 0.5, 0.25];
    let second = [0.0_f32, 0.5, 0.75];
    match require_deterministic(&first, &second) {
        Ok(()) => Err("the two renders differ, but the check called them equal".to_owned()),
        Err(ProfileError::Nondeterministic { sample: 2 }) => Ok(()),
        Err(error) => Err(format!(
            "the error is {error:?}, expected Nondeterministic at sample 2"
        )),
    }
}

#[test]
fn two_plain_renders_that_match_are_deterministic() -> Result<(), String> {
    let first = [0.0_f32, 0.5, 0.25];
    require_deterministic(&first, &first).map_err(|error| error.to_string())
}
