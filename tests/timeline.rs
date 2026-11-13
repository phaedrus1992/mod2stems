//! Tests the playback policy, the common timeline, and the mono downmix.
//!
//! The checks cover `docs/design.md` §4.2 (common timeline and subsong) and
//! §4.3 (stereo and mono). The vendored `test.mod` holds two subsongs, so it
//! can tell a selected subsong from the default.

use mod2stems::DEFAULT_MAX_RENDER_SECONDS;
use mod2stems::ProfileConfig;
use mod2stems::ProfileError;
use mod2stems::ProfileRequest;
use mod2stems::StemError;
use mod2stems::stems::ChannelLayout;
use mod2stems::stems::PlaybackPolicy;
use mod2stems::stems::Timeline;
use mod2stems::stems::downmix;
use mod2stems::stems::export_channel;
use mod2stems::stems::reference_render;

#[path = "support/fields.rs"]
mod fields;

use fields::load;

/// Resolves the default profile.
fn default_config() -> Result<ProfileConfig, String> {
    ProfileRequest::default()
        .resolve()
        .map_err(|error| error.to_string())
}

#[test]
fn a_render_that_reaches_the_limit_is_incomplete() -> Result<(), String> {
    // The limit is a policy value, not a constant. A render that reaches it
    // fails as incomplete, and that failure never reads as a finished song
    // (`docs/design.md` §4.2). A zero limit reaches the cap before any read.
    let config = default_config()?;
    let policy = PlaybackPolicy::default_policy().with_max_seconds(0);
    let mut module = load("test.mod")?;
    match reference_render(&mut module, &config, &policy) {
        Err(StemError::Profile(ProfileError::TooLong { limit_seconds: 0 })) => Ok(()),
        other => Err(format!(
            "a zero limit gives {other:?}, expected an incomplete render"
        )),
    }
}

#[test]
fn the_default_policy_plays_the_first_subsong() -> Result<(), String> {
    // `docs/design.md` §4.2 does not inherit the library's default selection.
    let policy = PlaybackPolicy::default_policy();
    if policy.subsong() != 0 {
        return Err(format!(
            "the default subsong is {}, expected 0",
            policy.subsong()
        ));
    }
    if policy.max_seconds() != DEFAULT_MAX_RENDER_SECONDS {
        return Err(format!(
            "the default limit is {} s, expected {DEFAULT_MAX_RENDER_SECONDS}",
            policy.max_seconds()
        ));
    }
    Ok(())
}

#[test]
fn a_policy_selects_the_subsong_and_plays_one_pass() -> Result<(), String> {
    let mut module = load("test.mod")?;
    PlaybackPolicy::default_policy()
        .with_subsong(1)
        .apply(&mut module)
        .map_err(|error| error.to_string())?;
    let selected = module
        .selected_subsong()
        .map_err(|error| error.to_string())?;
    if selected != 1 {
        return Err(format!("the selected subsong is {selected}, expected 1"));
    }
    let count = module.repeat_count().map_err(|error| error.to_string())?;
    if count != 0 {
        return Err(format!("the repeat count is {count}, expected 0"));
    }
    Ok(())
}

#[test]
fn a_policy_resets_a_repeat_count_that_the_module_carried() -> Result<(), String> {
    // A module that arrives with a repeat count would render the song again.
    // The policy owns the count, so it must set it on every apply.
    let mut module = load("test.mod")?;
    module
        .set_repeat_count(5)
        .map_err(|error| error.to_string())?;
    PlaybackPolicy::default_policy()
        .apply(&mut module)
        .map_err(|error| error.to_string())?;
    let count = module.repeat_count().map_err(|error| error.to_string())?;
    if count != 0 {
        return Err(format!("the repeat count is {count}, expected 0"));
    }
    Ok(())
}

#[test]
fn a_subsong_outside_the_module_fails() -> Result<(), String> {
    let mut module = load("test.mod")?;
    let subsongs = module.num_subsongs();
    match PlaybackPolicy::default_policy()
        .with_subsong(subsongs)
        .apply(&mut module)
    {
        Err(StemError::Profile(error)) => {
            let text = error.to_string();
            if !text.contains("subsong") {
                return Err(format!(
                    "the error is {text:?}, expected it to name the subsong"
                ));
            }
            Ok(())
        }
        other => Err(format!(
            "subsong {subsongs} gives {other:?}, expected a profile error"
        )),
    }
}

#[test]
fn the_common_end_is_the_longest_render() -> Result<(), String> {
    let short = [0.0_f32; 4];
    let long = [0.0_f32; 10];
    let timeline = Timeline::from_renders([&short[..], &long[..]]).map_err(|e| e.to_string())?;
    if timeline.frames() != 5 {
        return Err(format!(
            "the common end is {} frames, expected 5",
            timeline.frames()
        ));
    }
    Ok(())
}

#[test]
fn a_shorter_render_is_zero_padded() -> Result<(), String> {
    let long = [0.0_f32; 8];
    let timeline = Timeline::from_renders([&long[..]]).map_err(|e| e.to_string())?;
    let short = [0.25_f32, -0.25, 0.5, -0.5];
    let padded = timeline.pad(&short).map_err(|e| e.to_string())?;
    if padded.len() != 8 {
        return Err(format!(
            "the padded render holds {} samples, expected 8",
            padded.len()
        ));
    }
    // The head keeps the samples, so a padded track starts at the same origin.
    if padded[..4] != short {
        return Err(format!(
            "the head is {:?}, expected {:?}",
            &padded[..4],
            short
        ));
    }
    if padded[4..] != [0.0_f32; 4] {
        return Err(format!("the tail is {:?}, expected zeros", &padded[4..]));
    }
    Ok(())
}

#[test]
fn a_render_longer_than_the_timeline_fails() -> Result<(), String> {
    // Padding cannot shorten a render. A track past the common end means the
    // common end was computed from the wrong set, so the call must fail rather
    // than truncate the audio (`docs/design.md` §4.2).
    let short = [0.0_f32; 4];
    let timeline = Timeline::from_renders([&short[..]]).map_err(|e| e.to_string())?;
    let long = [0.0_f32; 8];
    match timeline.pad(&long) {
        Err(StemError::RenderPastTimeline { frames, timeline }) if frames == 4 && timeline == 2 => {
            Ok(())
        }
        other => Err(format!(
            "a longer render gives {other:?}, expected RenderPastTimeline"
        )),
    }
}

#[test]
fn padding_reports_the_frames_that_it_adds() -> Result<(), String> {
    // The caller reports the difference before it pads, so padding is never a
    // silent remedy for a divergent playthrough.
    let long = [0.0_f32; 10];
    let timeline = Timeline::from_renders([&long[..]]).map_err(|e| e.to_string())?;
    let short = [0.0_f32; 4];
    let padding = timeline.padding_for(&short).map_err(|e| e.to_string())?;
    if padding != 3 {
        return Err(format!("the padding is {padding} frames, expected 3"));
    }
    Ok(())
}

#[test]
fn an_odd_sample_count_fails() -> Result<(), String> {
    // The samples are interleaved stereo, so a half frame is a defect.
    let odd = [0.0_f32; 3];
    match Timeline::from_renders([&odd[..]]) {
        Err(StemError::NotStereo { samples: 3 }) => Ok(()),
        other => Err(format!("3 samples give {other:?}, expected NotStereo")),
    }
}

#[test]
fn an_empty_timeline_fails() -> Result<(), String> {
    let empty: [&[f32]; 0] = [];
    match Timeline::from_renders(empty) {
        Err(StemError::EmptyTimeline) => Ok(()),
        other => Err(format!("no renders give {other:?}, expected EmptyTimeline")),
    }
}

#[test]
fn a_padded_render_keeps_its_leading_silence() -> Result<(), String> {
    // Leading silence is audio, not a gap to trim. The first non-zero sample
    // must stay at the same index after alignment.
    let long = [0.0_f32; 12];
    let timeline = Timeline::from_renders([&long[..]]).map_err(|e| e.to_string())?;
    let mut short = vec![0.0_f32; 8];
    short[4] = 1.0;
    let padded = timeline.pad(&short).map_err(|e| e.to_string())?;
    if padded.first() != Some(&0.0) || padded.get(4) != Some(&1.0) {
        return Err(format!(
            "the padded render is {padded:?}, expected the 1.0 at index 4"
        ));
    }
    Ok(())
}

#[test]
fn mono_is_the_mean_of_the_two_channels() -> Result<(), String> {
    let stereo = [1.0_f32, 0.0, 0.5, -0.5];
    let mono = downmix(ChannelLayout::Mono, &stereo).map_err(|e| e.to_string())?;
    if mono != [0.5_f32, 0.0] {
        return Err(format!("the downmix is {mono:?}, expected [0.5, 0.0]"));
    }
    Ok(())
}

#[test]
fn stereo_keeps_the_samples_that_it_reads() -> Result<(), String> {
    let stereo = [1.0_f32, 0.0, 0.5, -0.5];
    let kept = downmix(ChannelLayout::Stereo, &stereo).map_err(|e| e.to_string())?;
    if kept != stereo {
        return Err(format!("stereo gives {kept:?}, expected the input"));
    }
    Ok(())
}

#[test]
fn a_mono_downmix_of_half_a_frame_fails() -> Result<(), String> {
    let odd = [0.0_f32; 3];
    match downmix(ChannelLayout::Mono, &odd) {
        Err(StemError::NotStereo { samples: 3 }) => Ok(()),
        other => Err(format!("3 samples give {other:?}, expected NotStereo")),
    }
}

#[test]
fn the_mono_sum_of_two_stems_equals_the_mono_of_their_sum() -> Result<(), String> {
    // Mono is the mean of the two channels, which is linear, so a mix of mono
    // stems must equal the mono of the summed stems (`docs/design.md` §4.3).
    // The tolerance is one float epsilon, because the two orders of addition
    // can round differently.
    let first = [0.25_f32, 0.5, 0.75, 1.0, -0.125, 0.375];
    let second = [0.125_f32, -0.25, 0.5, -1.0, 0.25, 0.125];
    let mut sum = Vec::with_capacity(first.len());
    for (a, b) in first.iter().zip(second.iter()) {
        sum.push(a + b);
    }
    let summed_mono = downmix(ChannelLayout::Mono, &sum).map_err(|e| e.to_string())?;
    let first_mono = downmix(ChannelLayout::Mono, &first).map_err(|e| e.to_string())?;
    let second_mono = downmix(ChannelLayout::Mono, &second).map_err(|e| e.to_string())?;
    for (index, (whole, part)) in summed_mono.iter().zip(first_mono.iter()).enumerate() {
        let mixed = part + second_mono.get(index).copied().unwrap_or_default();
        if (whole - mixed).abs() > f32::EPSILON {
            return Err(format!(
                "sample {index} is {whole} for the mono of the sum and {mixed} for the sum of the mono stems"
            ));
        }
    }
    Ok(())
}

#[test]
fn a_stem_of_another_subsong_fails() -> Result<(), String> {
    // The reference and the stem must play the same subsong. Two subsongs of
    // one module are two different songs, and a trace comparison across them
    // measures nothing.
    let config = default_config()?;
    let policy = PlaybackPolicy::default_policy();
    let mut plain = load("test.mod")?;
    let reference =
        reference_render(&mut plain, &config, &policy).map_err(|error| error.to_string())?;
    let mut isolated = load("test.mod")?;
    let other = PlaybackPolicy::default_policy().with_subsong(1);
    match export_channel(&mut isolated, &config, &other, 0, &reference) {
        Err(StemError::ReferenceMismatch {
            reference_profile, ..
        }) => {
            if !reference_profile.contains("subsong 0") {
                return Err(format!(
                    "the mismatch names {reference_profile:?}, expected it to name subsong 0"
                ));
            }
            Ok(())
        }
        other => Err(format!(
            "another subsong gives {other:?}, expected ReferenceMismatch"
        )),
    }
}
