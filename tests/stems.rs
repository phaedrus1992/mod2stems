//! Tests the per-channel isolation renders.
//!
//! The checks cover the silencing table from `docs/design.md` §4.1, the row
//! trace comparison, and one live render per silencing method. The vendored
//! `test.s3m` selects zero channel volume, and the vendored `test.mod` selects
//! channel mute.

use mod2stems::ProfileConfig;
use mod2stems::ProfileError;
use mod2stems::ProfileRequest;
use mod2stems::StemError;
use mod2stems::profile::apply::METADATA_TYPE;
use mod2stems::profile::apply::apply_profile;
use mod2stems::stems::PlaybackPolicy;
use mod2stems::stems::Position;
use mod2stems::stems::RowTrace;
use mod2stems::stems::Silencing;
use mod2stems::stems::export_channel;
use mod2stems::stems::read_guarded;
use mod2stems::stems::reference_render;
use mod2stems::stems::silencing_for;
use openmpt_ext::Module;

#[path = "support/fields.rs"]
mod fields;
#[path = "support/header.rs"]
mod header;
#[path = "support/s3m.rs"]
mod s3m;

use fields::load;
use fields::load_bytes;

/// Loads the module that `tests/support/s3m.rs` builds.
fn load_muted_flow() -> Result<Module, String> {
    load_bytes(&s3m::muted_flow_s3m())
}

/// Resolves the default profile.
fn default_config() -> Result<ProfileConfig, String> {
    ProfileRequest::default()
        .resolve()
        .map_err(|error| error.to_string())
}

/// Returns the default playback policy.
fn policy() -> PlaybackPolicy {
    PlaybackPolicy::default_policy()
}

/// Returns the silencing method for a type that must select one.
fn method(name: &str) -> Result<Silencing, String> {
    silencing_for(name).map_err(|error| error.to_string())
}

#[test]
fn st3_rule_types_select_zero_volume() -> Result<(), String> {
    // 669, FAR, and MTM reach the S3M rules through GetBestSaveFormat
    // (Sndfile.cpp:1433). C67, CBA, and ETX start from the S3M type itself.
    for name in [
        "s3m", "stm", "stx", "667", "ftm", "669", "far", "mtm", "c67", "cba", "etx", "fmt",
    ] {
        if method(name)? != Silencing::ZeroVolume {
            return Err(format!("{name} does not select zero channel volume"));
        }
    }
    Ok(())
}

#[test]
fn other_types_select_channel_mute() -> Result<(), String> {
    for name in ["mod", "it", "xm", "mptm", "med", "ult", "okt", "rtm"] {
        if method(name)? != Silencing::ChannelMute {
            return Err(format!("{name} does not select channel mute"));
        }
    }
    Ok(())
}

#[test]
fn a_type_with_two_rule_sets_fails() -> Result<(), String> {
    // amf, dsm, dmf, gdm, mo3, and psm each name more than one rule set, so the
    // token cannot decide the method. The check must fail, and it must not fall
    // back to the method that skips effects under ST3 rules.
    for name in ["amf", "dmf", "dsm", "gdm", "mo3", "psm"] {
        match silencing_for(name) {
            Err(StemError::UncertainRules { module_type }) if module_type == name => {}
            other => {
                return Err(format!("{name} gives {other:?}, expected UncertainRules"));
            }
        }
    }
    Ok(())
}

#[test]
fn the_type_token_ignores_case() -> Result<(), String> {
    if method("S3M")? != Silencing::ZeroVolume {
        return Err("an upper-case type token does not select zero channel volume".to_owned());
    }
    if method("Mod")? != Silencing::ChannelMute {
        return Err("a mixed-case type token does not select channel mute".to_owned());
    }
    Ok(())
}

#[test]
fn a_module_without_a_type_fails() -> Result<(), String> {
    for name in ["", "   "] {
        match silencing_for(name) {
            Err(StemError::Profile(ProfileError::MissingModuleType)) => {}
            other => {
                return Err(format!(
                    "{name:?} gives {other:?}, expected MissingModuleType"
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn a_position_lands_once_per_transition() -> Result<(), String> {
    let mut trace = RowTrace::default();
    for row in [0, 0, 0, 1, 1, 2, 0] {
        trace.record(Position::new(0, row));
    }
    let wanted = [
        Position::new(0, 0),
        Position::new(0, 1),
        Position::new(0, 2),
        Position::new(0, 0),
    ];
    if trace.positions() != wanted {
        return Err(format!(
            "the trace holds {:?}, expected {wanted:?}",
            trace.positions()
        ));
    }
    Ok(())
}

/// Builds a trace that visits the named rows of order 0.
fn trace_of(rows: &[i32]) -> RowTrace {
    let mut trace = RowTrace::default();
    for row in rows {
        trace.record(Position::new(0, *row));
    }
    trace
}

#[test]
fn equal_traces_do_not_diverge() -> Result<(), String> {
    let one = trace_of(&[0, 1]);
    match one.first_divergence(&trace_of(&[0, 1])) {
        None => Ok(()),
        Some(index) => Err(format!("equal traces diverge at position {index}")),
    }
}

#[test]
fn a_different_position_reports_its_index() -> Result<(), String> {
    match trace_of(&[0, 1]).first_divergence(&trace_of(&[0, 2])) {
        Some(1) => Ok(()),
        other => Err(format!("the traces report {other:?}, expected Some(1)")),
    }
}

#[test]
fn a_shorter_trace_diverges_at_its_length() -> Result<(), String> {
    match trace_of(&[0]).first_divergence(&trace_of(&[0, 1])) {
        Some(1) => Ok(()),
        other => Err(format!("the traces report {other:?}, expected Some(1)")),
    }
}

#[test]
fn the_fixture_selects_the_s3m_rules() -> Result<(), String> {
    let module = load_muted_flow()?;
    if module.num_channels() != s3m::CHANNELS {
        return Err(format!(
            "the fixture declares {} channels, expected {}",
            module.num_channels(),
            s3m::CHANNELS
        ));
    }
    let module_type = module
        .metadata(METADATA_TYPE)
        .map_err(|error| error.to_string())?;
    if module_type.is_empty() {
        return Err("the fixture reports an empty type, so a renamed key would pass".to_owned());
    }
    match silencing_for(&module_type).map_err(|error| error.to_string())? {
        Silencing::ZeroVolume => Ok(()),
        Silencing::ChannelMute => Err(format!(
            "the fixture type {module_type:?} selects channel mute, expected zero channel volume"
        )),
    }
}

#[test]
fn channel_mute_drops_the_flow_of_a_s3m_module() -> Result<(), String> {
    // This test states the reason that the S3M types use zero volume. Channel
    // mute skips every effect on a muted channel under ST3 rules, so the
    // pattern break on channel 1 never runs.
    let config = default_config()?;
    let mut plain = load_muted_flow()?;
    let reference =
        reference_render(&mut plain, &config, &policy()).map_err(|error| error.to_string())?;
    let mut muted = load_muted_flow()?;
    apply_profile(&config, &mut muted).map_err(|error| error.to_string())?;
    for channel in 1..muted.num_channels() {
        muted
            .set_channel_mute(channel, true)
            .map_err(|error| error.to_string())?;
    }
    let render = read_guarded(&mut muted, config.rate(), policy().max_seconds())
        .map_err(|error| error.to_string())?;
    match render.trace.first_divergence(reference.trace()) {
        Some(_) => Ok(()),
        None => Err(
            "channel mute keeps the flow of the S3M fixture, and the fixture then does not test the ST3 rule"
                .to_owned(),
        ),
    }
}

#[test]
fn zero_volume_keeps_the_flow_of_a_s3m_module() -> Result<(), String> {
    let config = default_config()?;
    let mut plain = load_muted_flow()?;
    let reference =
        reference_render(&mut plain, &config, &policy()).map_err(|error| error.to_string())?;
    let mut isolated = load_muted_flow()?;
    let export = export_channel(&mut isolated, &config, &policy(), 0, &reference)
        .map_err(|error| error.to_string())?;
    if &export.trace != reference.trace() {
        return Err(format!(
            "the isolated render of the fixture diverges at {:?}",
            export.trace.first_divergence(reference.trace())
        ));
    }
    Ok(())
}

#[test]
fn a_reference_from_another_module_fails() -> Result<(), String> {
    // A trace from another module compares two unrelated songs, so the flow
    // check must refuse it before it silences a channel.
    let config = default_config()?;
    let mut other = load("test.mod")?;
    let reference =
        reference_render(&mut other, &config, &policy()).map_err(|error| error.to_string())?;
    let mut isolated = load_muted_flow()?;
    match export_channel(&mut isolated, &config, &policy(), 0, &reference) {
        Err(StemError::ReferenceMismatch {
            module_type,
            reference_type,
            ..
        }) if module_type == "s3m" && reference_type == "mod" => Ok(()),
        other => Err(format!(
            "the export gives {other:?}, expected ReferenceMismatch"
        )),
    }
}

#[test]
fn a_reference_from_another_amiga_model_fails() -> Result<(), String> {
    // The model is part of the settings, not of the profile name. Two
    // references can both name the `amiga` profile and still render with
    // different filters, so the export must refuse the pair.
    let a500 = ProfileRequest::amiga(Some(mod2stems::AmigaModel::A500))
        .resolve()
        .map_err(|error| error.to_string())?;
    let a1200 = ProfileRequest::amiga(Some(mod2stems::AmigaModel::A1200))
        .resolve()
        .map_err(|error| error.to_string())?;
    let mut module = load("test.mod")?;
    let reference =
        reference_render(&mut module, &a500, &policy()).map_err(|error| error.to_string())?;
    let mut isolated = load("test.mod")?;
    match export_channel(&mut isolated, &a1200, &policy(), 0, &reference) {
        Err(StemError::ReferenceMismatch {
            profile,
            reference_profile,
            ..
        }) if profile.contains("a1200") && reference_profile.contains("a500") => Ok(()),
        other => Err(format!(
            "the export gives {other:?}, expected ReferenceMismatch"
        )),
    }
}

#[test]
fn channel_mute_stems_add_up_to_the_mix() -> Result<(), String> {
    stems_add_up_to_the_mix("test.mod")
}

#[test]
fn zero_volume_stems_add_up_to_the_mix() -> Result<(), String> {
    stems_add_up_to_the_mix("test.s3m")
}

/// Renders every channel of one module, and sums the stems.
///
/// The sum must equal the unmuted render sample for sample. That proves the
/// silencing removed exactly the audio of the other channels, and that no
/// channel render changed the song flow.
fn stems_add_up_to_the_mix(name: &str) -> Result<(), String> {
    let config = default_config()?;
    let mut plain = load(name)?;
    let reference =
        reference_render(&mut plain, &config, &policy()).map_err(|error| error.to_string())?;
    let channels = plain.num_channels();
    let mut sum = vec![0.0_f32; reference.samples().len()];
    for channel in 0..channels {
        // The loop bound comes from one instance and the render runs on
        // another. `export_channel` compares the second instance's count with
        // the reference, so a differing count fails there.
        let mut isolated = load(name)?;
        let exported = export_channel(&mut isolated, &config, &policy(), channel, &reference)
            .map_err(|error| format!("{name} channel {channel}: {error}"))?;
        for (part, stem) in sum.iter_mut().zip(exported.samples.iter()) {
            *part += stem;
        }
    }
    // The comparison is bit-exact on purpose. Each stem is a deterministic
    // libopenmpt pass, so the sum must reproduce the mix to the last bit. A
    // tolerance would hide a stem that leaks another channel.
    match sum
        .iter()
        .zip(reference.samples().iter())
        .position(|(part, whole)| part.to_bits() != whole.to_bits())
    {
        None => Ok(()),
        Some(index) => Err(format!(
            "the sum of the {channels} stems of {name} differs from the mix at sample {index}: {} against {}",
            sum.get(index).copied().unwrap_or_default(),
            reference.samples().get(index).copied().unwrap_or_default()
        )),
    }
}

#[test]
fn a_channel_outside_the_module_fails() -> Result<(), String> {
    let config = default_config()?;
    let mut module = load("test.mod")?;
    let channels = module.num_channels();
    let mut plain = load("test.mod")?;
    let reference =
        reference_render(&mut plain, &config, &policy()).map_err(|error| error.to_string())?;
    match export_channel(&mut module, &config, &policy(), channels, &reference) {
        Err(StemError::ChannelOutOfRange {
            channel,
            channels: found,
        }) if channel == channels && found == channels => Ok(()),
        other => Err(format!(
            "channel {channels} gives {other:?}, expected ChannelOutOfRange"
        )),
    }
}

#[test]
fn a_silenced_channel_that_raises_its_volume_mid_song_fails() -> Result<(), String> {
    // The fixture sets the silenced channel volume back to zero before the song
    // ends, so a check that runs only after the render reads zero and passes.
    // The export must catch the raise while the song runs.
    let config = default_config()?;
    let mut plain = load_bytes(&s3m::volume_swing_s3m())?;
    let reference =
        reference_render(&mut plain, &config, &policy()).map_err(|error| error.to_string())?;
    let mut isolated = load_bytes(&s3m::volume_swing_s3m())?;
    match export_channel(&mut isolated, &config, &policy(), 0, &reference) {
        Err(StemError::VolumeDrifted {
            exported: 0,
            silenced: 1,
            ..
        }) => Ok(()),
        other => Err(format!(
            "the export gives {other:?}, expected VolumeDrifted on channel 1"
        )),
    }
}

#[test]
fn a_reference_from_another_profile_fails() -> Result<(), String> {
    // The flow check measures one performance. A reference rendered under
    // another profile is a different performance, so the export refuses it
    // before it compares any trace.
    let compatible = default_config()?;
    let clean = ProfileRequest::clean()
        .resolve()
        .map_err(|error| error.to_string())?;
    let mut module = load_muted_flow()?;
    let reference =
        reference_render(&mut module, &compatible, &policy()).map_err(|error| error.to_string())?;
    let mut isolated = load_muted_flow()?;
    match export_channel(&mut isolated, &clean, &policy(), 0, &reference) {
        Err(StemError::ReferenceMismatch {
            profile,
            reference_profile,
            ..
        }) if profile.starts_with("the clean profile")
            && reference_profile.starts_with("the compatible profile") =>
        {
            Ok(())
        }
        other => Err(format!(
            "the export gives {other:?}, expected ReferenceMismatch"
        )),
    }
}
