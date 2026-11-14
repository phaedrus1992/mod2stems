//! Property tests for the manifest, the name sanitizer, and the file names.
//!
//! The example tests in `tests/manifest.rs` pin the named cases. These tests
//! cover the shapes that a fixed case cannot: any module text, any pair of
//! channel indices, and any manifest value.
//!
//! A module is untrusted input, so the sanitizer properties are the security
//! bound of the export (`docs/design.md` §5.2).

use std::path::PathBuf;

use mod2stems::manifest::ExportDetails;
use mod2stems::manifest::Manifest;
use mod2stems::manifest::StemEntry;
use mod2stems::manifest::file_name;
use mod2stems::manifest::pair_file_name;
use mod2stems::manifest::sanitize;
use proptest::collection::vec;
use proptest::prelude::any;
use proptest::prelude::prop_assert;
use proptest::prelude::prop_assert_eq;
use proptest::prelude::prop_assert_ne;
use proptest::proptest;
use proptest::test_runner::Config;

/// Returns true when a sanitized name is safe to place in a file name.
fn is_safe(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || character == '_'
                || character == '-'
        })
        && !name.starts_with('_')
        && !name.ends_with('_')
}

proptest! {
    #![proptest_config(Config::with_cases(128))]

    /// Any module text gives either no name or a safe one.
    #[test]
    fn a_sanitized_name_holds_only_safe_characters(name in any::<String>()) {
        if let Some(clean) = sanitize(&name) {
            prop_assert!(is_safe(&clean), "the name {name:?} gave {clean:?}");
        }
    }

    /// Sanitizing a sanitized name changes nothing.
    #[test]
    fn sanitize_is_idempotent(name in any::<String>()) {
        if let Some(clean) = sanitize(&name) {
            prop_assert_eq!(sanitize(&clean), Some(clean));
        }
    }

    /// Two channels that share a name still get two file names.
    #[test]
    fn a_channel_index_keeps_two_names_apart(
        first in 0_i32..1024,
        second in 0_i32..1024,
        name in any::<String>(),
        extension in "[a-z]{2,4}",
    ) {
        if first == second {
            return Ok(());
        }
        let one = file_name(first, Some(&name), &extension);
        let other = file_name(second, Some(&name), &extension);
        prop_assert_ne!(one, other);
    }

    /// Two channels that share a slot still get two file names.
    #[test]
    fn a_pair_channel_index_keeps_two_names_apart(
        first in 0_i32..1024,
        second in 0_i32..1024,
        slot in 0_i32..1024,
    ) {
        if first == second {
            return Ok(());
        }
        let one = pair_file_name(first, slot, Some("Kick"), "wav");
        let other = pair_file_name(second, slot, Some("Kick"), "wav");
        prop_assert_ne!(one, other);
    }

    /// Any manifest value survives the JSON round trip, in stem order.
    #[test]
    fn a_manifest_survives_the_json_round_trip(
        gain in -8.0_f32..8.0,
        settings in vec((any::<String>(), any::<String>()), 0..4),
        metadata in vec((any::<String>(), any::<String>()), 0..4),
        warnings in vec(any::<String>(), 0..4),
        mix in proptest::option::of(any::<String>()),
        stems in 0_usize..8,
    ) {
        let manifest = Manifest::new(
            details(gain, settings, metadata, warnings, mix),
            entries(stems),
        )
        .map_err(|error| proptest::test_runner::TestCaseError::fail(error.to_string()))?;
        let json = manifest
            .to_json()
            .map_err(|error| proptest::test_runner::TestCaseError::fail(error.to_string()))?;
        let read = Manifest::from_json(&json)
            .map_err(|error| proptest::test_runner::TestCaseError::fail(error.to_string()))?;
        prop_assert_eq!(&read, &manifest);
        prop_assert_eq!(read.stems(), manifest.stems());
    }
}

/// Returns the export details of one generated manifest.
fn details(
    gain: f32,
    settings: Vec<(String, String)>,
    metadata: Vec<(String, String)>,
    warnings: Vec<String>,
    mix: Option<String>,
) -> ExportDetails {
    ExportDetails {
        input_hash: "sha256:00".to_owned(),
        tool_version: "0.1.0".to_owned(),
        libopenmpt_version: "0.8.10+r25840".to_owned(),
        profile: "compatible".to_owned(),
        settings,
        sample_rate: 48_000,
        layout: "stereo".to_owned(),
        frame_count: 4,
        end_policy: "song end".to_owned(),
        gain,
        mix,
        reference_mix: None,
        format: "float WAV".to_owned(),
        dither: "none".to_owned(),
        quantization: "none (float)".to_owned(),
        subsong: 0,
        module_metadata: metadata,
        warnings,
        validation: "unverified".to_owned(),
    }
}

/// Returns `count` distinct channel entries, in channel order.
fn entries(count: usize) -> Vec<StemEntry> {
    // A count that does not fit a channel index names no stem, so it gives an
    // empty set rather than a wrapped index.
    let count = i32::try_from(count).unwrap_or(0);
    (0..count)
        .map(|index| {
            StemEntry::channel(
                index,
                Some(format!("channel {index}")),
                PathBuf::from(format!("ch_{index:02}.wav")),
            )
        })
        .collect()
}
