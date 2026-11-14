//! The two export sets that one export can write.
//!
//! A channel stem and the pair stems of that channel both hold the same audio
//! at two granularities. A reader that imports both sets sums each channel
//! twice, so the export labels them as alternatives (`docs/design.md` §5.2).

use serde::Deserialize;
use serde::Serialize;

/// One set of files that a single export writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportSet {
    /// One file per tracker channel.
    Channels,
    /// One file per channel and instrument pair.
    Pairs,
}

impl ExportSet {
    /// Returns the directory of the set, relative to the export root.
    #[must_use]
    pub fn directory(self) -> Option<&'static str> {
        match self {
            Self::Channels => None,
            Self::Pairs => Some(super::PAIR_DIRECTORY),
        }
    }

    /// Returns the text that names the set to a reader.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Channels => "channel stems",
            Self::Pairs => "instrument/channel pair stems",
        }
    }
}
