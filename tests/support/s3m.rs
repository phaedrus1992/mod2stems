//! Builds one small `S3M` module whose flow command sits on channel 1.
//!
//! Channel 0 plays a note. Channel 1 carries one pattern break and nothing
//! else. Under `ScreamTracker 3` rules libopenmpt skips every effect on a muted
//! channel (`Snd_fx.cpp:581`, `3238`), so a channel-muted render drops the
//! pattern break and the song flow changes. A zeroed channel volume does not
//! trip the rule, so the break runs.
//!
//! This module is the only fixture that shows the two silencing methods apart.
//! The vendored corpus has no module with an effect on a channel that a render
//! silences.
//!
//! The builder writes the bytes in the test process, so the fixture lives in
//! this repository. The vendor tree is a git submodule, and a file written
//! there would not survive a submodule update.
//!
//! Layout: the header, the order list, the sample and pattern parapointer
//! tables, the sample header, the pattern, and the sample data. Every
//! parapointer names a 16-byte paragraph.

/// Byte length of the S3M file header (`S3MTools.h`).
const HEADER_LEN: usize = 96;
/// Byte length of one S3M sample header (`S3MTools.h`).
const SAMPLE_HEADER_LEN: usize = 80;
/// Byte length of one parapointer table entry.
const PARAPOINTER_LEN: usize = 2;
/// Byte length of the paragraph that the parapointers count in.
const PARAGRAPH: usize = 16;
/// Number of channels that the module declares.
const CHANNELS: i32 = 2;
/// Number of orders in the fixture module.
const ORDERS: usize = 2;
/// Number of patterns in the fixture module.
const PATTERNS: usize = 1;
/// Rows in every pattern. The loader always inserts 64 rows.
const ROWS: usize = 64;
/// The byte that ends a row in a packed S3M pattern.
const END_OF_ROW: u8 = 0x00;
/// Length of the sample data, in samples.
const SAMPLE_DATA_LEN: usize = 64;
/// Middle-C frequency of the sample, in Hz.
const C5SPEED: u32 = 8363;
/// The row that the pattern break jumps to, in the S3M decimal-nibble form.
///
/// `S3MConvert` reads the low nibble as the low decimal digit, so `0x20` is
/// row 20 (`Load_s3m.cpp:40`).
const BREAK_ROW: u8 = 0x20;
/// The note on the first row of the pattern.
///
/// The loader maps a value below `0xF0` to `(note & 0x0F) + 12 * (note >> 4) +
/// 12 + NOTE_MIN` (`Load_s3m.cpp:684`).
const NOTE: u8 = 0x40;
/// The instrument slot that the note plays.
const INSTRUMENT: u8 = 1;
/// The `cwtv` value that selects the `ScreamTracker` branch.
///
/// The tracker probe rejects its special cases only when the ultra-click field
/// is zero (`Load_s3m.cpp:262-300`), so this value with the ultra-click value
/// below keeps `kST3NoMutedChannels`.
const CWT_V: u16 = 0x1320;
/// The ultra-click value that keeps the file out of the special cases.
const ULTRA_CLICKS: u8 = 16;
/// Offset of the sample parapointer table.
const SMP_POINTERS_AT: usize = HEADER_LEN + ORDERS;
/// Offset of the pattern parapointer table.
const PAT_POINTERS_AT: usize = SMP_POINTERS_AT + PARAPOINTER_LEN;
/// Offset of the sample header, on a paragraph boundary.
const SAMPLE_AT: usize = 8 * PARAGRAPH;

// A drift in the layout constants stops the build instead of writing a file
// whose tables point at the wrong bytes.
const _: () = assert!(SMP_POINTERS_AT == 98);
const _: () = assert!(PAT_POINTERS_AT + PARAPOINTER_LEN <= SAMPLE_AT);
const _: () = assert!(SAMPLE_AT.is_multiple_of(PARAGRAPH));

/// Returns the bytes of the fixture module.
///
/// Both orders play pattern 0. Order 0 breaks to row 20, so order 1 starts
/// there. A render that misses the break plays all 64 rows of both orders and
/// visits a different sequence.
#[must_use]
pub fn muted_flow_s3m() -> Vec<u8> {
    let packed = pattern();
    let pattern_at = SAMPLE_AT + SAMPLE_HEADER_LEN;
    let data_at = pattern_at + packed.len();
    let mut file = vec![0_u8; SAMPLE_AT];
    file[..HEADER_LEN].copy_from_slice(&header());
    file[HEADER_LEN..HEADER_LEN + ORDERS].copy_from_slice(&[0_u8, 0]);
    let sample_pointer = paragraph(SAMPLE_AT);
    file[SMP_POINTERS_AT..SMP_POINTERS_AT + PARAPOINTER_LEN].copy_from_slice(&sample_pointer);
    let pattern_pointer = paragraph(pattern_at);
    file[PAT_POINTERS_AT..PAT_POINTERS_AT + PARAPOINTER_LEN].copy_from_slice(&pattern_pointer);
    file.extend_from_slice(&sample_header(data_at));
    file.extend_from_slice(&packed);
    file.extend_from_slice(&sample_data());
    file
}

/// Returns the number of channels that the fixture module declares.
#[must_use]
pub fn channels() -> i32 {
    CHANNELS
}

/// Returns the paragraph that holds one absolute offset.
fn paragraph(offset: usize) -> [u8; PARAPOINTER_LEN] {
    let value = u16::try_from(offset / PARAGRAPH).unwrap_or(u16::MAX);
    value.to_le_bytes()
}

/// Writes the file header.
fn header() -> [u8; HEADER_LEN] {
    let mut out = [0_u8; HEADER_LEN];
    out[0..11].copy_from_slice(b"MutedFlow\0\0");
    out[28] = 0x1A;
    out[29] = 0x10;
    out[32..34].copy_from_slice(&u16_len(ORDERS).to_le_bytes());
    out[34..36].copy_from_slice(&u16_len(1).to_le_bytes());
    out[36..38].copy_from_slice(&u16_len(PATTERNS).to_le_bytes());
    out[40..42].copy_from_slice(&CWT_V.to_le_bytes());
    // Format version 1 means the sample data is signed 8-bit.
    out[42..44].copy_from_slice(&1_u16.to_le_bytes());
    out[44..48].copy_from_slice(b"SCRM");
    out[48] = 64;
    out[49] = 6;
    out[50] = 125;
    // The high bit makes the module stereo, which the loader accepts.
    out[51] = 0xB0;
    out[52] = ULTRA_CLICKS;
    // Channel 0 is left, channel 1 is right, and every other slot is unused.
    out[64] = 0x00;
    out[65] = 0x08;
    for byte in &mut out[66..96] {
        *byte = 0xFF;
    }
    out
}

/// Writes the sample header for the one sample.
fn sample_header(data_at: usize) -> [u8; SAMPLE_HEADER_LEN] {
    let mut out = [0_u8; SAMPLE_HEADER_LEN];
    out[0] = 1;
    out[1..5].copy_from_slice(b"wave");
    let pointer = paragraph(data_at);
    out[13] = pointer[0];
    out[14] = pointer[1];
    out[16..20].copy_from_slice(&u32_len(SAMPLE_DATA_LEN).to_le_bytes());
    out[20..24].copy_from_slice(&0_u32.to_le_bytes());
    out[24..28].copy_from_slice(&u32_len(SAMPLE_DATA_LEN).to_le_bytes());
    out[28] = 64;
    // The loop flag is set, so the note sustains across the whole pattern.
    out[31] = 0x01;
    out[32..36].copy_from_slice(&C5SPEED.to_le_bytes());
    out[48..52].copy_from_slice(b"wave");
    out[76..80].copy_from_slice(b"SCRS");
    out
}

/// Writes the packed pattern.
///
/// Row 0 plays the note on channel 0, and breaks on channel 1. Every later row
/// is empty.
fn pattern() -> Vec<u8> {
    let mut cells = vec![0x20, NOTE, INSTRUMENT, END_OF_ROW];
    cells.extend_from_slice(&[0x80 | 1, b'C', BREAK_ROW, END_OF_ROW]);
    cells.extend(std::iter::repeat_n(END_OF_ROW, ROWS - 2));
    let mut out = u16_len(cells.len()).to_le_bytes().to_vec();
    out.extend_from_slice(&cells);
    while !out.len().is_multiple_of(PARAGRAPH) {
        out.push(0);
    }
    out
}

/// Writes the sample data, which is a square wave.
fn sample_data() -> Vec<u8> {
    (0..SAMPLE_DATA_LEN)
        .map(|index| if index % 8 < 4 { 96_u8 } else { 160_u8 })
        .collect()
}

/// Converts a count to a 16-bit header field.
fn u16_len(value: usize) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

/// Converts a count to a 32-bit header field.
fn u32_len(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}
