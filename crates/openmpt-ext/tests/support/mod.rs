//! Builds `ProTracker` `MOD` files for the tests.

/// One pattern cell.
pub mod cell;
/// One sample header and its data.
pub mod sample;

pub use cell::Cell;
pub use sample::SampleSpec;

/// A whole MOD file.
pub struct ModSpec {
    /// Pattern rows. The builder pads the pattern to 64 rows.
    pub rows: Vec<[Cell; 4]>,
    /// Sample headers, 1 to 31.
    pub samples: Vec<SampleSpec>,
    /// Order list. The builder pads it to 128 entries.
    pub orders: Vec<u8>,
}

/// Number of sample header slots in a MOD file.
const SAMPLE_SLOTS: usize = 31;
/// Number of rows in a pattern.
const ROWS_PER_PATTERN: usize = 64;
/// Number of channels in the test files.
const CHANNELS: usize = 4;
/// Length of the 128-entry order list.
const ORDER_SLOTS: usize = 128;
/// Byte length of one sample header.
const SAMPLE_HEADER_LEN: usize = 30;
/// Offset of the magic bytes in a MOD file.
const MAGIC_OFFSET: usize = 1080;

/// Builds the file bytes for one MOD.
pub fn build_mod(spec: &ModSpec) -> Vec<u8> {
    build_with_magic(spec, *b"M.K.")
}

/// Builds the file bytes with the given four magic bytes.
pub fn build_with_magic(spec: &ModSpec, magic: [u8; 4]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend([b' '; 20]);
    for slot in 0..SAMPLE_SLOTS {
        write_sample_header(&mut out, spec.samples.get(slot));
    }
    write_order_list(&mut out, &spec.orders);
    out.extend(magic);
    write_pattern(&mut out, &spec.rows);
    for sample in &spec.samples {
        for value in &sample.data {
            out.push(value.cast_unsigned());
        }
    }
    debug_assert_eq!(out[MAGIC_OFFSET..MAGIC_OFFSET + 4], magic);
    out
}

/// Writes one 30-byte sample header, or a zero slot when the sample is absent.
///
/// The header holds a word count, so an odd byte count has no exact value. The
/// builder rejects it here, because a truncated count would make the loader
/// read fewer bytes than the file holds.
fn write_sample_header(out: &mut Vec<u8>, sample: Option<&SampleSpec>) {
    let Some(sample) = sample else {
        out.extend([0; SAMPLE_HEADER_LEN]);
        return;
    };
    let bytes = sample.data.len();
    assert!(
        bytes.is_multiple_of(2),
        "sample data holds {bytes} bytes, and a MOD sample holds whole words"
    );
    out.extend(sample.name);
    out.extend(words(bytes / 2).to_be_bytes());
    out.push(0);
    out.push(sample.volume);
    out.extend(sample.loop_start.to_be_bytes());
    out.extend(sample.loop_len.max(1).to_be_bytes());
}

/// Converts a byte count to a saturating 16-bit word count.
fn words(bytes: usize) -> u16 {
    u16::try_from(bytes).unwrap_or(u16::MAX)
}

/// Writes the song length, the restart byte, and 128 order entries.
fn write_order_list(out: &mut Vec<u8>, orders: &[u8]) {
    let length = u8::try_from(orders.len().clamp(1, ORDER_SLOTS)).unwrap_or(1);
    out.push(length);
    out.push(127);
    for index in 0..ORDER_SLOTS {
        out.push(orders.get(index).copied().unwrap_or(0));
    }
}

/// Writes all 64 rows of the pattern, with four channels each.
fn write_pattern(out: &mut Vec<u8>, rows: &[[Cell; 4]]) {
    for row in 0..ROWS_PER_PATTERN {
        for channel in 0..CHANNELS {
            let cell = rows.get(row).and_then(|cells| cells.get(channel));
            write_cell(out, cell);
        }
    }
}

/// Returns the low 8 bits of a period. The high nibble goes in the first byte.
fn low_byte(value: u16) -> u8 {
    u8::try_from(value & 0xFF).unwrap_or(0)
}

/// Writes one cell, or an empty cell when the position has no data.
fn write_cell(out: &mut Vec<u8>, cell: Option<&Cell>) {
    let Some(cell) = cell else {
        out.extend([0; 4]);
        return;
    };
    let period_hi = u8::try_from(cell.period >> 8).unwrap_or(0) & 0x0F;
    out.push((cell.sample & 0xF0) | period_hi);
    out.push(low_byte(cell.period));
    out.push(((cell.sample & 0x0F) << 4) | (cell.effect & 0x0F));
    out.push(cell.param);
}

/// Builds a valid MOD that plays four samples.
///
/// The file has one pattern, two orders that both play pattern 0, and a note on
/// channel `c` for sample `c + 1` at row 0.
pub fn simple_mod() -> Vec<u8> {
    let names: [&[u8]; 4] = [b"kick", b"snare", b"hat", b"bass"];
    let samples = names
        .iter()
        .map(|name| sample_with_name(name))
        .collect::<Vec<_>>();
    let first_row = [0, 1, 2, 3].map(|channel| Cell {
        period: 428,
        sample: channel + 1,
        effect: 0,
        param: 0,
    });
    build_mod(&ModSpec {
        rows: vec![first_row],
        samples,
        orders: vec![0, 0],
    })
}

/// Returns a sample with a given name and 256 bytes of non-zero data.
pub fn sample_with_name(name: &[u8]) -> SampleSpec {
    let mut padded = [0; 22];
    let len = name.len().min(padded.len());
    padded[..len].copy_from_slice(&name[..len]);
    SampleSpec {
        name: padded,
        data: vec![0x20; 256],
        volume: 64,
        loop_start: 0,
        loop_len: 0,
    }
}
