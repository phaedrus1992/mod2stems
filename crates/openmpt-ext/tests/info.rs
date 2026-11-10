//! Tests the library info and the module metadata.

mod support;

use openmpt_ext::LibraryInfo;
use openmpt_ext::Module;
use openmpt_ext::library_info;

use support::Cell;
use support::ModSpec;
use support::SampleSpec;

/// Loads the simple test file.
fn load() -> Result<Module, String> {
    Module::from_bytes(&support::simple_mod(), &[]).map_err(|error| error.to_string())
}

#[test]
fn library_version_is_0_8_10() -> Result<(), String> {
    // The full string appends the source revision, such as `0.8.10+r25840`, so
    // only the version prefix is stable.
    let version = library_info(LibraryInfo::LibraryVersion);
    if !version.starts_with("0.8.10") {
        return Err(format!(
            "library_version is {version:?}, expected it to start with \"0.8.10\""
        ));
    }
    Ok(())
}

#[test]
fn source_revision_is_pinned() -> Result<(), String> {
    let revision = library_info(LibraryInfo::SourceRevision);
    if revision != openmpt_ext_sys::PIN_SVN_REVISION {
        return Err(format!(
            "source_revision is {revision:?}, expected {:?}",
            openmpt_ext_sys::PIN_SVN_REVISION
        ));
    }
    Ok(())
}

#[test]
fn metadata_keys_include_type_and_title() -> Result<(), String> {
    let keys = load()?.metadata_keys();
    for wanted in ["type", "title"] {
        if !keys.iter().any(|key| key == wanted) {
            return Err(format!("metadata_keys has no {wanted:?}: {keys:?}"));
        }
    }
    Ok(())
}

#[test]
fn metadata_type_is_mod() -> Result<(), String> {
    let module = load()?;
    let value = module.metadata("type").map_err(|error| error.to_string())?;
    if value != "mod" {
        return Err(format!("type is {value:?}, expected \"mod\""));
    }
    Ok(())
}

#[test]
fn counts_match_simple_mod() -> Result<(), String> {
    let module = load()?;
    // A sample header with length zero is still a sample slot, and the ProTracker
    // loader fills all 31 of them.
    let checks = [
        ("channels", module.num_channels(), 4),
        ("instruments", module.num_instruments(), 0),
        ("samples", module.num_samples(), 31),
        ("subsongs", module.num_subsongs(), 1),
    ];
    for (name, actual, expected) in checks {
        if actual != expected {
            return Err(format!("{name} is {actual}, expected {expected}"));
        }
    }
    Ok(())
}

#[test]
fn sample_name_round_trips() -> Result<(), String> {
    let module = load()?;
    let name = module.sample_name(0).map_err(|error| error.to_string())?;
    if name != "kick" {
        return Err(format!("sample 0 is {name:?}, expected \"kick\""));
    }
    Ok(())
}

#[test]
fn high_name_byte_is_transcoded() -> Result<(), String> {
    // libopenmpt transcodes a module string to UTF-8 before the wrapper reads it,
    // so a high byte arrives as a valid character and never as a broken sequence.
    let names: [&[u8]; 4] = [b"kick", b"snare", b"hat", b"bass"];
    let mut samples: Vec<SampleSpec> = names
        .iter()
        .map(|name| SampleSpec {
            name: padded(name),
            data: vec![0x20; 256],
            volume: 64,
            loop_start: 0,
            loop_len: 0,
        })
        .collect();
    if let Some(first) = samples.first_mut() {
        first.name[0] = 0xFF;
    }
    let bytes = support::build_mod(&ModSpec {
        rows: vec![[note(1), note(2), note(3), note(4)]],
        samples,
        orders: vec![0],
    });
    let module = Module::from_bytes(&bytes, &[]).map_err(|error| error.to_string())?;
    let name = module.sample_name(0).map_err(|error| error.to_string())?;
    if name != "\u{FF}ick" {
        return Err(format!("sample 0 is {name:?}, expected \"\\u{{FF}}ick\""));
    }
    Ok(())
}

#[test]
fn channel_name_out_of_range() -> Result<(), String> {
    let module = load()?;
    let error = module.channel_name(4).err();
    let Some(error) = error else {
        return Err("channel 4 must be out of range".to_owned());
    };
    if error.input != "channel 4" {
        return Err(format!(
            "input is {:?}, expected \"channel 4\"",
            error.input
        ));
    }
    if !error.message.contains("0..4") {
        return Err(format!(
            "message is {:?}, expected it to name 0..4",
            error.message
        ));
    }
    Ok(())
}

#[test]
fn metadata_unknown_key_is_empty_string() -> Result<(), String> {
    let module = load()?;
    let value = module
        .metadata("no.such.key")
        .map_err(|error| error.to_string())?;
    if !value.is_empty() {
        return Err(format!("an unknown key gave {value:?}, expected \"\""));
    }
    Ok(())
}

/// Returns a cell that plays one sample.
fn note(sample: u8) -> Cell {
    Cell {
        period: 428,
        sample,
        effect: 0,
        param: 0,
    }
}

/// Copies a name into a 22-byte field.
fn padded(name: &[u8]) -> [u8; 22] {
    let mut padded = [0; 22];
    let len = name.len().min(padded.len());
    padded[..len].copy_from_slice(&name[..len]);
    padded
}
