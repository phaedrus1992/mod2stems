//! Property tests for the timeline, the mono downmix, and the policy builder.
//!
//! The example tests in `tests/timeline.rs` pin the named cases. These tests
//! cover the shapes that a fixed case cannot: any sample pattern, any render
//! length, and any subsong value.
//!
//! Every buffer holds whole stereo frames. A generated sample is compared by
//! its bits, because a NaN is not equal to itself and the renderer can write
//! one.

use mod2stems::StemError;
use mod2stems::stems::ChannelLayout;
use mod2stems::stems::PlaybackPolicy;
use mod2stems::stems::Timeline;
use mod2stems::stems::downmix;
use proptest::collection::vec;
use proptest::prelude::any;
use proptest::prelude::prop_assert;
use proptest::prelude::prop_assert_eq;
use proptest::proptest;
use proptest::test_runner::Config;
use proptest::test_runner::TestCaseError;

#[path = "support/fields.rs"]
mod fields;

use fields::load;

/// Builds a timeline whose length is `frames` frames.
fn timeline_of(frames: usize) -> Result<Timeline, String> {
    let samples = vec![0.0_f32; frames.saturating_mul(2)];
    Timeline::from_renders([&samples[..]]).map_err(|error| error.to_string())
}

/// Returns the bit patterns of one sample slice.
fn bits(samples: &[f32]) -> Vec<u32> {
    samples.iter().map(|sample| sample.to_bits()).collect()
}

proptest! {
    #![proptest_config(Config::with_cases(64))]

    /// The common end is the longest render, in frames.
    #[test]
    fn the_timeline_is_the_longest_render(
        first_frames in 1_usize..64,
        second_frames in 1_usize..64,
    ) {
        let first = vec![0.0_f32; first_frames * 2];
        let second = vec![0.0_f32; second_frames * 2];
        let timeline = Timeline::from_renders([&first[..], &second[..]])
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        prop_assert_eq!(timeline.frames(), first_frames.max(second_frames));
    }

    /// Padding keeps the head, zero-fills the tail, and reaches the timeline.
    #[test]
    fn padding_keeps_the_head_and_fills_the_tail(
        frames in 0_usize..32,
        values in vec(any::<f32>(), 64),
        extra_frames in 1_usize..32,
    ) {
        let samples: Vec<f32> = (0..frames)
            .flat_map(|frame| [values[frame], values[frame + 32]])
            .collect();
        let timeline = timeline_of(frames + extra_frames)
            .map_err(TestCaseError::fail)?;
        let padded = timeline
            .pad(&samples)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        prop_assert_eq!(padded.len(), timeline.frames() * 2);
        prop_assert_eq!(bits(&padded[..samples.len()]), bits(&samples[..]));
        prop_assert!(padded[samples.len()..].iter().all(|sample| *sample == 0.0));
    }

    /// Padding is idempotent, and it adds the count that padding_for reports.
    #[test]
    fn padding_agrees_with_the_reported_count(
        frames in 0_usize..32,
        values in vec(any::<f32>(), 64),
        extra_frames in 1_usize..32,
    ) {
        let samples: Vec<f32> = (0..frames)
            .flat_map(|frame| [values[frame], values[frame + 32]])
            .collect();
        let timeline = timeline_of(frames + extra_frames)
            .map_err(TestCaseError::fail)?;
        let reported = timeline
            .padding_for(&samples)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        let padded = timeline
            .pad(&samples)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        prop_assert_eq!(padded.len() - samples.len(), reported * 2);
        let again = timeline
            .pad(&padded)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        prop_assert_eq!(bits(&again), bits(&padded));
    }

    /// A render past the timeline fails in both calls, not just in pad.
    #[test]
    fn a_render_past_the_timeline_fails_both_calls(
        frames in 2_usize..33,
        timeline_frames in 1_usize..32,
    ) {
        if frames <= timeline_frames {
            return Ok(());
        }
        let samples = vec![0.0_f32; frames * 2];
        let timeline = timeline_of(timeline_frames)
            .map_err(TestCaseError::fail)?;
        let expected = StemError::RenderPastTimeline {
            frames,
            timeline: timeline_frames,
        };
        match (timeline.padding_for(&samples), timeline.pad(&samples)) {
            (Err(reported), Err(padded)) => {
                prop_assert_eq!(reported, expected.clone());
                prop_assert_eq!(padded, expected);
            }
            (reported, padded) => {
                return Err(TestCaseError::fail(format!(
                    "{frames} frames on a {timeline_frames}-frame timeline gives {reported:?} and {padded:?}"
                )));
            }
        }
    }

    /// Stereo keeps the samples that it reads.
    #[test]
    fn stereo_downmixes_to_itself(
        frames in 0_usize..32,
        values in vec(any::<f32>(), 64),
    ) {
        let samples: Vec<f32> = (0..frames)
            .flat_map(|frame| [values[frame], values[frame + 32]])
            .collect();
        let kept = downmix(ChannelLayout::Stereo, &samples)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        prop_assert_eq!(bits(&kept), bits(&samples));
    }

    /// Mono holds one sample per frame, and each is the mean of its pair.
    #[test]
    fn mono_is_the_mean_of_each_pair(
        frames in 0_usize..32,
        values in vec(any::<f32>(), 64),
    ) {
        let samples: Vec<f32> = (0..frames)
            .flat_map(|frame| [values[frame], values[frame + 32]])
            .collect();
        let mono = downmix(ChannelLayout::Mono, &samples)
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        prop_assert_eq!(mono.len(), frames);
        for (index, pair) in samples.as_chunks::<2>().0.iter().enumerate() {
            let [left, right] = *pair;
            prop_assert_eq!(bits(&[mono[index]]), bits(&[f32::midpoint(left, right)]));
        }
    }

    /// The policy keeps both values through either builder order.
    #[test]
    fn the_policy_keeps_both_values(
        subsong in any::<i32>(),
        limit in any::<u64>(),
    ) {
        let policy = PlaybackPolicy::default_policy()
            .with_subsong(subsong)
            .with_max_seconds(limit);
        prop_assert_eq!(policy.subsong(), subsong);
        prop_assert_eq!(policy.max_seconds(), limit);
        let reversed = PlaybackPolicy::default_policy()
            .with_max_seconds(limit)
            .with_subsong(subsong);
        prop_assert_eq!(reversed, policy);
    }

    /// A subsong outside the module fails, and an accepted one reads back.
    #[test]
    fn the_policy_either_fails_or_reads_back(
        subsong in -4_i32..8,
    ) {
        let mut module = load("test.mod")
            .map_err(TestCaseError::fail)?;
        let subsongs = module.num_subsongs();
        match PlaybackPolicy::default_policy().with_subsong(subsong).apply(&mut module) {
            Ok(()) => {
                prop_assert!((0..subsongs).contains(&subsong));
                let selected = module.selected_subsong()
                    .map_err(|error| TestCaseError::fail(error.to_string()))?;
                prop_assert_eq!(selected, subsong);
                let count = module.repeat_count()
                    .map_err(|error| TestCaseError::fail(error.to_string()))?;
                prop_assert_eq!(count, 0);
            }
            Err(StemError::Profile(_)) => prop_assert!(!(0..subsongs).contains(&subsong)),
            Err(other) => {
                return Err(TestCaseError::fail(format!(
                    "subsong {subsong} gives {other}"
                )));
            }
        }
    }
}
