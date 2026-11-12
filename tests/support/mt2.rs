//! Builds one small `MT2` module whose instrument keeps the nearest mode.
//!
//! The MT2 sample path sets `SRCMODE_NEAREST` on an instrument when its sample
//! carries the "no interpolation" flag (`Load_mt2.cpp:1062`, `1107`). A
//! per-instrument resampling mode wins over the global interpolation mode
//! (`Sndmix.cpp:2564-2571`), so the `clean` profile cannot render this module as
//! named. The vendor corpus carries no MT2 file, so this builder gives the tests
//! a real one.
//!
//! The layout follows `Load_mt2.cpp`: the file header, the order table, the drum
//! and extra-data lengths, one pattern, 255 instrument entries, 256 sample
//! entries, one sample group, and no sample data. Every entry must be present,
//! because the loader always reads that many.
//!
//! The vendor tree is a git submodule, so a fixture written there would not
//! survive an update. The builder writes the bytes in the test process.

use crate::fields::u32_field;

/// Byte length of the MT2 file header (`MT2FileHeader`).
const HEADER_LEN: usize = 126;
/// Byte length of one `MT2Instrument` header.
const INSTRUMENT_LEN: usize = 106;
/// Byte length of the instrument flags field.
const INSTRUMENT_FLAGS_LEN: usize = 2;
/// Byte length of the instrument envelope mask.
const ENVELOPE_MASK_LEN: usize = 4;
/// Byte length of one `MT2Sample` header.
const SAMPLE_LEN: usize = 26;
/// Byte length of one `MT2Group` entry.
const GROUP_LEN: usize = 8;
/// Number of notes in the instrument keyboard map.
const GROUP_MAP_LEN: usize = 96;
/// Byte length of a name field in an instrument or sample entry.
const NAME_LEN: usize = 32;
/// The loader reads 255 instrument entries and 256 sample entries, whatever
/// the header counts say (`Load_mt2.cpp:886`, `1003`).
const INSTRUMENT_ENTRIES: usize = 255;
const SAMPLE_ENTRIES: usize = 256;
/// The number of orders in the fixture module.
const ORDERS: u16 = 1;
/// The byte length of the order table that the loader always reads.
const ORDER_TABLE_LEN: usize = 256;
/// The header version. It must be in `0x200..0x300` (`Load_mt2.cpp:394`), and
/// `0x0202` makes the loader read the envelope mask.
const VERSION: u16 = 0x0202;
/// The sample "no interpolation" flag (`Load_mt2.cpp:1055`).
const SAMPLE_NO_INTERPOLATION: u8 = 8;
/// The header flag that selects packed patterns (`Load_mt2.cpp:26`).
const PACKED_PATTERNS: u32 = 0x01;
/// The sample rate that the fixture sample reports.
const C5SPEED: u32 = 8363;

/// Returns the bytes of the MT2 fixture module.
///
/// The module has one channel, one instrument, and one sample. The sample
/// carries the "no interpolation" flag, and the instrument maps it to every
/// note, so the loader sets the nearest resampling mode on the instrument.
#[must_use]
pub fn nearest_resampling_mt2() -> Vec<u8> {
    let mut file = Vec::new();
    file.extend_from_slice(&header());
    // The loader always reads the full order table, whatever `numOrders` says.
    file.extend_from_slice(&[0_u8; ORDER_TABLE_LEN]);
    // No drum channels, and no extra data chunk.
    file.extend_from_slice(&0_u16.to_le_bytes());
    file.extend_from_slice(&0_u32.to_le_bytes());
    file.extend_from_slice(&pattern());
    file.extend_from_slice(&instrument_entries());
    file.extend_from_slice(&sample_entries());
    file.extend_from_slice(&group());
    file
}

/// Writes the MT2 file header.
fn header() -> [u8; HEADER_LEN] {
    let mut out = [0_u8; HEADER_LEN];
    out[0..4].copy_from_slice(b"MT20");
    out[8..10].copy_from_slice(&VERSION.to_le_bytes());
    copy_text(&mut out[10..42], b"MadTracker 2.0");
    copy_text(&mut out[42..106], b"Nearest");
    out[106..108].copy_from_slice(&ORDERS.to_le_bytes()); // orders
    out[110..112].copy_from_slice(&1_u16.to_le_bytes()); // patterns
    out[112..114].copy_from_slice(&1_u16.to_le_bytes()); // channels
    out[116] = 6; // ticks per line
    out[117] = 4; // lines per beat
    out[118..122].copy_from_slice(&PACKED_PATTERNS.to_le_bytes());
    out[122..124].copy_from_slice(&1_u16.to_le_bytes()); // instruments
    out[124..126].copy_from_slice(&1_u16.to_le_bytes()); // samples
    out
}

/// Writes the single pattern: one row, one channel, and no command.
///
/// The packed form reads one info byte per cell, and a zero byte names no field
/// (`Load_mt2.cpp:520`). The chunk length rounds up to an even number.
fn pattern() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&1_u16.to_le_bytes()); // rows
    out.extend_from_slice(&2_u32.to_le_bytes()); // chunk length
    out.extend_from_slice(&[0_u8, 0_u8]);
    out
}

/// Writes all 255 instrument entries.
///
/// Entry 0 holds the instrument header, a zero flags field, and an envelope
/// mask of zero. The other entries hold an empty name and no data.
fn instrument_entries() -> Vec<u8> {
    let mut out = Vec::with_capacity(INSTRUMENT_ENTRIES * (NAME_LEN + 4));
    let mut name = [0_u8; NAME_LEN];
    copy_text(&mut name, b"Instrument");
    out.extend_from_slice(&name);
    let length = INSTRUMENT_LEN + INSTRUMENT_FLAGS_LEN + ENVELOPE_MASK_LEN;
    out.extend_from_slice(&u32_field(length).to_le_bytes());
    out.extend_from_slice(&instrument_header());
    out.extend_from_slice(&[0_u8; INSTRUMENT_FLAGS_LEN]);
    out.extend_from_slice(&[0_u8; ENVELOPE_MASK_LEN]);
    for _ in 1..INSTRUMENT_ENTRIES {
        out.extend_from_slice(&[0_u8; NAME_LEN]);
        out.extend_from_slice(&0_u32.to_le_bytes());
    }
    out
}

/// Writes the instrument header: one sample group, mapped to every note.
fn instrument_header() -> [u8; INSTRUMENT_LEN] {
    let mut out = [0_u8; INSTRUMENT_LEN];
    out[0..2].copy_from_slice(&1_u16.to_le_bytes()); // one sample group
    // Every note maps to group 0, which holds sample 0.
    out[2..2 + GROUP_MAP_LEN].fill(0);
    out
}

/// Writes all 256 sample entries.
///
/// Entry 0 holds the sample header with the "no interpolation" flag. The other
/// entries hold an empty name and no data.
fn sample_entries() -> Vec<u8> {
    let mut out = Vec::with_capacity(SAMPLE_ENTRIES * (NAME_LEN + 4));
    let mut name = [0_u8; NAME_LEN];
    copy_text(&mut name, b"nearest");
    out.extend_from_slice(&name);
    out.extend_from_slice(&u32_field(SAMPLE_LEN).to_le_bytes());
    out.extend_from_slice(&sample_header());
    for _ in 1..SAMPLE_ENTRIES {
        out.extend_from_slice(&[0_u8; NAME_LEN]);
        out.extend_from_slice(&0_u32.to_le_bytes());
    }
    out
}

/// Writes the sample header with the "no interpolation" flag.
///
/// The length stays zero, so the loader reads no sample bytes.
fn sample_header() -> [u8; SAMPLE_LEN] {
    let mut out = [0_u8; SAMPLE_LEN];
    out[4..8].copy_from_slice(&C5SPEED.to_le_bytes());
    out[8] = 1; // depth: 8-bit
    out[9] = 1; // channels: mono
    out[10] = SAMPLE_NO_INTERPOLATION;
    out[20..22].copy_from_slice(&128_u16.to_le_bytes()); // volume
    out
}

/// Writes the one sample group, which points at sample 0.
fn group() -> [u8; GROUP_LEN] {
    let mut out = [0_u8; GROUP_LEN];
    out[0] = 0; // sample 0
    out[1] = 64; // volume
    out
}

/// Copies text into a fixed-size field, and pads the rest with zero bytes.
fn copy_text(field: &mut [u8], text: &[u8]) {
    let take = text.len().min(field.len());
    field[..take].copy_from_slice(&text[..take]);
}
