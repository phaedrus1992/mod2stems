//! The identity of one file in an export.
//!
//! The identity says which slot a file came from. The sanitized name is text
//! for a person and can repeat, so the identity decides what the file is.

use serde::Deserialize;
use serde::Serialize;

/// The identity of one file in an export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Identity {
    /// A whole tracker channel.
    Channel,
    /// One instrument of one channel.
    Instrument {
        /// The instrument slot.
        instrument: i32,
    },
    /// One sample of one channel, for a module that has samples and no
    /// instruments.
    Sample {
        /// The sample slot.
        sample: i32,
    },
}
