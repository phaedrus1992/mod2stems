//! One sample header and its data.

/// One sample header and its data.
pub struct SampleSpec {
    /// Sample name, padded with zeros.
    pub name: [u8; 22],
    /// Signed 8-bit sample data.
    pub data: Vec<i8>,
    /// Playback volume, 0 to 64.
    pub volume: u8,
    /// Loop start in words.
    pub loop_start: u16,
    /// Loop length in words. The file stores a minimum of 1.
    pub loop_len: u16,
}
