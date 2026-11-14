//! What one finished export reports.

use std::path::PathBuf;

/// The result of one published export.
///
/// The value names what the export wrote, so the command prints a result that a
/// person can check without reading the staging directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportReport {
    /// The published export directory.
    pub destination: PathBuf,
    /// The number of renders that the export ran.
    pub renders: usize,
    /// The audio files that the export wrote.
    pub files: usize,
    /// The frame count that every audio file spans.
    pub frames: usize,
}
