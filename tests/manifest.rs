//! Tests the export layout, the file names, and the manifest.
//!
//! The checks cover the name sanitizing, the collision rule, the manifest
//! fields, and the staged publication.

use std::fmt::Debug;
use std::path::PathBuf;

use mod2stems::manifest::ExportDetails;
use mod2stems::manifest::ExportSet;
use mod2stems::manifest::Manifest;
use mod2stems::manifest::ManifestError;
use mod2stems::manifest::StagingReport;
use mod2stems::manifest::StemEntry;
use mod2stems::manifest::file_name;
use mod2stems::manifest::pair_file_name;
use mod2stems::manifest::sample_file_name;
use mod2stems::manifest::sanitize;

/// Returns an error that names `what` when `condition` is false.
fn require(condition: bool, what: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(what.to_owned())
    }
}

/// Returns an error that names both values when they differ.
fn require_eq<T: Debug + PartialEq>(actual: &T, expected: &T, what: &str) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{what}: got {actual:?}, expected {expected:?}"))
    }
}

#[test]
fn a_plain_name_keeps_its_text() {
    assert_eq!(sanitize("lead synth").as_deref(), Some("lead_synth"));
}

#[test]
fn a_path_separator_never_reaches_a_file_name() -> Result<(), String> {
    // A name from the module is untrusted. A separator or a traversal would
    // put the file outside the export directory (docs/design.md §5.2).
    for name in ["a/b", "a\\b", "../../etc/passwd", "..", "."] {
        // A name with no usable character gives no text at all, which is the
        // safe answer. A name with one gives text with no separator in it.
        if let Some(clean) = sanitize(name) {
            require(
                !clean.contains('/') && !clean.contains('\\') && !clean.contains(".."),
                &format!("the name {name:?} gave {clean:?}"),
            )?;
        }
    }
    Ok(())
}

#[test]
fn a_name_with_no_usable_character_gives_none() {
    // An empty name would give a file called `ch_03_.wav`, which collides with
    // every other empty name, and `..` could name the parent directory.
    for name in ["///", "..", "."] {
        assert_eq!(sanitize(name), None, "the name {name:?} must give None");
    }
}

#[test]
fn a_platform_invalid_character_is_replaced() -> Result<(), String> {
    // Windows rejects these in a file name, and a macOS export that a Windows
    // user cannot open is a defect.
    let clean = sanitize("a:b*c?d\"e<f>g|h").ok_or("a run of letters gives a name")?;
    for bad in [':', '*', '?', '"', '<', '>', '|'] {
        require(
            !clean.contains(bad),
            &format!("the name still holds {bad:?}"),
        )?;
    }
    Ok(())
}

#[test]
fn a_long_name_stays_inside_the_limit() -> Result<(), String> {
    // A file name has a per-component limit on every platform, and a stem name
    // comes from the module. The sanitizer caps it, and the index still names
    // the file.
    let long = "x".repeat(500);
    let clean = sanitize(&long).ok_or("a run of letters gives a name")?;
    require(
        clean.len() <= 64,
        &format!("the name is {} bytes long", clean.len()),
    )
}

#[test]
fn a_channel_file_name_holds_the_index_and_the_name() {
    assert_eq!(file_name(3, Some("Lead"), "wav"), "ch_03_lead.wav");
    assert_eq!(file_name(3, None, "wav"), "ch_03.wav");
}

#[test]
fn two_channels_with_the_same_name_get_different_files() {
    // The index is authoritative and the name is not unique, so the name can
    // never decide the path.
    assert_ne!(
        file_name(1, Some("Lead"), "wav"),
        file_name(2, Some("Lead"), "wav")
    );
}

#[test]
fn a_pair_file_name_holds_both_indices() {
    assert_eq!(
        pair_file_name(3, 7, Some("Kick"), "flac"),
        "ch_03_ins_007_kick.flac"
    );
}

#[test]
fn a_sample_only_pair_uses_a_sample_marker() {
    // A sample-only module has no instrument, so the identity must say which
    // slot the number names (docs/design.md §5.2).
    assert_eq!(
        sample_file_name(3, 7, Some("Kick"), "wav"),
        "ch_03_smp_007_kick.wav"
    );
}

#[test]
fn the_manifest_records_the_required_fields() -> Result<(), String> {
    let manifest = manifest()?;
    let json = manifest.to_json().map_err(|e| e.to_string())?;
    for key in [
        "input_hash",
        "tool_version",
        "libopenmpt_version",
        "profile",
        "sample_rate",
        "layout",
        "frame_count",
        "end_policy",
        "gain",
        "format",
        "dither",
        "stems",
        "validation",
    ] {
        require(
            json.contains(key),
            &format!("the manifest does not name {key}"),
        )?;
    }
    Ok(())
}

#[test]
fn the_manifest_round_trips() -> Result<(), String> {
    let manifest = manifest()?;
    let json = manifest.to_json().map_err(|e| e.to_string())?;
    let read = Manifest::from_json(&json).map_err(|e| e.to_string())?;
    require_eq(&read, &manifest, "the manifest survives a round trip")
}

#[test]
fn a_manifest_with_a_colliding_path_fails() -> Result<(), String> {
    // Two stems that name one file would overwrite each other, and the export
    // would publish a set with a missing stem and no message.
    let Err(error) = Manifest::new(details(), vec![stem(1, "Lead"), stem(1, "Lead")]) else {
        return Err("a collision must fail".to_owned());
    };
    require(
        matches!(error, ManifestError::Collision { .. }),
        &format!("the error must name the collision, got {error:?}"),
    )
}

#[test]
fn the_manifest_names_the_staging_directory_and_the_retained_files() -> Result<(), String> {
    // A failed export reports the files that it kept, so nobody deletes a good
    // staging directory by accident and nobody leaves one unnoticed
    // (docs/design.md §5.2).
    let report = StagingReport::new(PathBuf::from("/tmp/stage"));
    require(
        report.to_string().contains("/tmp/stage"),
        "the report names the path",
    )?;
    let kept = report.with_retained(vec!["ch_01.wav".to_owned()]);
    require(
        kept.to_string().contains("ch_01.wav"),
        "the report names the retained file",
    )
}

#[test]
fn a_channel_with_one_pair_reuses_the_channel_stem() {
    // The design allows the reuse, and the manifest must say that it happened
    // (docs/design.md §4.5).
    let entry = stem(1, "Lead").reused();
    assert_eq!(entry.set(), ExportSet::Channels);
    assert!(entry.is_reused());
}

/// Returns one stem entry.
fn stem(channel: i32, name: &str) -> StemEntry {
    StemEntry::channel(
        channel,
        Some(name.to_owned()),
        PathBuf::from(format!("ch_{channel}.wav")),
    )
}

/// Returns one complete manifest.
fn manifest() -> Result<Manifest, String> {
    Manifest::new(details(), vec![stem(1, "Lead")]).map_err(|error| error.to_string())
}

/// Returns the export details that one manifest records.
fn details() -> ExportDetails {
    ExportDetails {
        input_hash: "sha256:00".to_owned(),
        tool_version: "0.1.0".to_owned(),
        libopenmpt_version: "0.8.10+r25840".to_owned(),
        profile: "compatible".to_owned(),
        settings: vec![("play.at_end".to_owned(), "stop".to_owned())],
        sample_rate: 48_000,
        layout: "stereo".to_owned(),
        frame_count: 48_000,
        end_policy: "song end".to_owned(),
        gain: 1.0,
        mix: None,
        reference_mix: None,
        format: "float WAV".to_owned(),
        dither: "none".to_owned(),
        quantization: "none".to_owned(),
        subsong: 0,
        module_metadata: vec![("type".to_owned(), "mod".to_owned())],
        warnings: Vec::new(),
        validation: "unverified".to_owned(),
    }
}
