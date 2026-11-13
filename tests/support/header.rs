//! Converts counts to header fields of the module builders.
//!
//! A test binary that mounts only `fields.rs` has no builder and no use for
//! these helpers, so they live in their own file.

/// Converts a byte count to a 16-bit header field.
///
/// Each caller passes a compile-time length that fits, so the clamp never runs.
#[must_use]
pub fn u16_field(value: usize) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

/// Converts a byte count to a 32-bit header field.
///
/// Each caller passes a compile-time length that fits, so the clamp never runs.
#[must_use]
pub fn u32_field(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}
