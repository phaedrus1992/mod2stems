//! Builds one small `IT` module whose instrument reads the random generator.
//!
//! The module has one instrument with a volume swing, and one pattern that
//! plays a note on one channel. The volume of every processed tick then depends
//! on `mpt::random<int8>(AccessPRNG())` (`Snd_fx.cpp:2041`). Each `CSoundFile`
//! seeds that generator from the process-wide one (`Sndfile.cpp:121`), so two
//! instances of this module render differently.
//!
//! The builder writes the bytes in the test process, so the fixture lives in
//! this repository. `crates/openmpt-ext-sys/vendor/openmpt` is a git submodule,
//! and a file written there would not survive a submodule update.
//!
//! The layout follows `ITTools.h`: the file header, the order list, three
//! parapointer tables, the instrument, the sample header, the pattern, and the
//! sample data.

/// Byte length of the IT file header.
const HEADER_LEN: usize = 192;
/// Byte length of one IT instrument header.
const INSTRUMENT_LEN: usize = 554;
/// Byte length of one IT sample header.
const SAMPLE_LEN: usize = 80;
/// Byte length of one parapointer table entry.
const PARAPOINTER_LEN: usize = 4;
/// Number of orders in the fixture module.
const ORDERS: usize = 1;
/// Number of instruments in the fixture module.
const INSTRUMENTS: usize = 1;
/// Number of samples in the fixture module.
const SAMPLES: usize = 1;
/// Number of patterns in the fixture module.
const PATTERNS: usize = 1;
/// Rows in the single pattern.
const ROWS: usize = 8;
/// Sample rate of the fixture sample, in Hz.
const C5SPEED: u32 = 8363;
/// Length of the sample data, in samples.
const SAMPLE_DATA_LEN: usize = 256;
/// Byte value of the note on row 0. The loader adds `NOTE_MIN` to a value below
/// `0x80` (`Load_it.cpp:1105`).
const NOTE: u8 = 60;
/// Volume swing that makes every triggered note read the random generator.
///
/// The loader caps the field at 100 (`ITTools.cpp:273`), so this value survives
/// the load unchanged.
const VOLUME_SWING: u8 = 64;
/// Number of keyboard map entries in an IT instrument.
const KEYBOARD_ENTRIES: usize = 120;
/// Offset of the sample data inside the sample, which starts at the sample
/// header offset as a little-endian 32-bit value.
const SAMPLE_POINTER_FIELD: usize = 72;
/// Offset of the instrument parapointer table.
const INS_POINTERS_AT: usize = HEADER_LEN + ORDERS;
/// Offset of the sample parapointer table.
const SMP_POINTERS_AT: usize = INS_POINTERS_AT + INSTRUMENTS * PARAPOINTER_LEN;
/// Offset of the pattern parapointer table.
const PAT_POINTERS_AT: usize = SMP_POINTERS_AT + SAMPLES * PARAPOINTER_LEN;
/// Offset of the instrument header.
const INSTRUMENT_AT: usize = PAT_POINTERS_AT + PATTERNS * PARAPOINTER_LEN;
/// Offset of the sample header.
const SAMPLE_AT: usize = INSTRUMENT_AT + INSTRUMENT_LEN;
/// Offset of the pattern.
const PATTERN_AT: usize = SAMPLE_AT + SAMPLE_LEN;

/// Returns the bytes of the randomized fixture module.
#[must_use]
pub fn randomized_it() -> Vec<u8> {
    let pattern = pattern();
    let data_at = PATTERN_AT + pattern.len();
    let mut file = Vec::with_capacity(data_at + SAMPLE_DATA_LEN);
    file.extend_from_slice(&header());
    file.extend(std::iter::repeat_n(0_u8, ORDERS));
    file.extend_from_slice(&parapointer(INSTRUMENT_AT));
    file.extend_from_slice(&parapointer(SAMPLE_AT));
    file.extend_from_slice(&parapointer(PATTERN_AT));
    file.extend_from_slice(&instrument());
    file.extend_from_slice(&sample_header(data_at));
    file.extend_from_slice(&pattern);
    file.extend_from_slice(&sample_data());
    file
}

/// Converts a byte count to a 16-bit header field.
fn u16_field(value: usize) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

/// Converts a byte count to a 32-bit header field.
fn u32_field(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

/// Writes one parapointer as a little-endian 32-bit offset.
fn parapointer(offset: usize) -> [u8; PARAPOINTER_LEN] {
    u32_field(offset).to_le_bytes()
}

/// Writes the IT file header.
fn header() -> [u8; HEADER_LEN] {
    let mut out = [0_u8; HEADER_LEN];
    out[0..4].copy_from_slice(b"IMPM");
    out[4..14].copy_from_slice(b"RandomVol\0");
    out[32..34].copy_from_slice(&u16_field(ORDERS).to_le_bytes());
    out[34..36].copy_from_slice(&u16_field(INSTRUMENTS).to_le_bytes());
    out[36..38].copy_from_slice(&u16_field(SAMPLES).to_le_bytes());
    out[38..40].copy_from_slice(&u16_field(PATTERNS).to_le_bytes());
    // cwtv names OpenMPT 1.32, and cmwt 0x0214 selects the current instrument
    // format. A cwtv of 0x0204 or 0x0214 trips a compatibility probe in the
    // loader, and this value stays clear of both.
    out[40..42].copy_from_slice(&0x1320_u16.to_le_bytes());
    out[42..44].copy_from_slice(&0x0214_u16.to_le_bytes());
    // flags: stereo playback, instrument mode, and linear slides.
    out[44..46].copy_from_slice(&0x000D_u16.to_le_bytes());
    out[48] = 128;
    out[49] = 48;
    out[50] = 6;
    out[51] = 125;
    out[52] = 128;
    // Channel 0 is centred, at full volume, and not muted.
    out[64] = 32;
    out[128] = 64;
    out
}

/// Writes the instrument header.
///
/// The volume swing is the field that makes the module read the generator.
/// Every other field keeps the value of a default instrument.
fn instrument() -> [u8; INSTRUMENT_LEN] {
    let mut out = [0_u8; INSTRUMENT_LEN];
    out[0..4].copy_from_slice(b"IMPI");
    out[4..12].copy_from_slice(b"Random\0\0");
    // ppc: the panning centre, which the panning field overrides.
    out[23] = 32;
    // gbv: 128 is full global volume.
    out[24] = 128;
    // dfp: 32 is centre panning, with the panning applied.
    out[25] = 32;
    // rv is the volume swing (`ITTools.cpp:273`).
    out[26] = VOLUME_SWING;
    out[28..30].copy_from_slice(&0x5000_u16.to_le_bytes());
    // nos: the number of embedded samples.
    out[30] = 1;
    out[32..42].copy_from_slice(b"Random\0\0\0\0");
    // Every note maps to sample 1, and every note keeps its own pitch.
    for entry in 0..KEYBOARD_ENTRIES {
        let note = u8::try_from(entry).unwrap_or(u8::MAX);
        out[64 + entry * 2] = note;
        out[64 + entry * 2 + 1] = 1;
    }
    out
}

/// Writes the sample header.
///
/// `data_at` is the absolute file offset of the sample data. The loader reads
/// the sample pointer, so the data need not follow the header.
fn sample_header(data_at: usize) -> [u8; SAMPLE_LEN] {
    let mut out = [0_u8; SAMPLE_LEN];
    out[0..4].copy_from_slice(b"IMPS");
    out[4..8].copy_from_slice(b"wave");
    // gvl: full global volume.
    out[17] = 64;
    // flags: the sample data is present, and the sample loops.
    out[18] = 0x11;
    // vol: full sample volume.
    out[19] = 64;
    out[20..24].copy_from_slice(b"wave");
    // cvt: the sample data is signed 8-bit.
    out[46] = 0x01;
    // dfp: centre panning.
    out[47] = 32;
    out[48..52].copy_from_slice(&u32_field(SAMPLE_DATA_LEN).to_le_bytes());
    out[52..56].copy_from_slice(&0_u32.to_le_bytes());
    out[56..60].copy_from_slice(&u32_field(SAMPLE_DATA_LEN).to_le_bytes());
    out[60..64].copy_from_slice(&C5SPEED.to_le_bytes());
    out[SAMPLE_POINTER_FIELD..SAMPLE_POINTER_FIELD + PARAPOINTER_LEN]
        .copy_from_slice(&parapointer(data_at));
    // The auto-vibrato fields stay zero, so the volume swing is the only random
    // source in the module.
    out
}

/// Writes the single pattern.
///
/// Four rows carry a note and an instrument on channel 0, and each of them
/// draws a new swing value. The rows sit two apart, so the note has time to
/// play between triggers. A zero byte ends a row, and the first byte of a cell
/// sets the channel with the mask-present flag (`Load_it.cpp:1077-1092`).
fn pattern() -> Vec<u8> {
    let mut data = Vec::with_capacity(ROWS * 4);
    for row in 0..ROWS {
        if row % 2 == 0 {
            // Channel 0, with a mask byte that names a note and an instrument.
            data.push(0x80 | 0x01);
            data.push(0x03);
            data.push(NOTE);
            data.push(1);
        } else {
            data.push(0);
        }
    }
    let mut out = Vec::with_capacity(8 + data.len());
    out.extend_from_slice(&u16_field(data.len()).to_le_bytes());
    out.extend_from_slice(&u16_field(ROWS).to_le_bytes());
    out.extend_from_slice(&0_u32.to_le_bytes());
    out.extend_from_slice(&data);
    out
}

/// Writes signed 8-bit sample data: a square wave, so every tick has a sample.
fn sample_data() -> Vec<u8> {
    (0..SAMPLE_DATA_LEN)
        .map(|index| if index % 8 < 4 { 48_u8 } else { 208_u8 })
        .collect()
}
