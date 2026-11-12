//! The silencing method that one module type needs.
//!
//! A per-channel render must silence every other channel and still keep the
//! timing and global effects that those channels carry. The stock API has two
//! methods, and neither covers every type (`docs/design.md` §4.1).
//!
//! The library does not expose the playback rules that decide this. The only
//! fact that the C API gives is the module type token, so the token selects the
//! method. Design: `docs/design.md` §4.1.

use crate::stems::StemError;

/// How a profile render silences the channels that it does not export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Silencing {
    /// Set the channel volume to zero.
    ///
    /// A channel-volume command can raise the value again, so the caller must
    /// read the volume back after the render.
    ZeroVolume,
    /// Set the channel mute flag.
    ///
    /// This also mutes the new-note-action voices that the channel owns.
    ChannelMute,
}

/// The module types that skip effects on a muted channel.
///
/// `SetDefaultPlaybackBehaviour` gives the rule to every module of the S3M type
/// (`Sndfile.cpp:1160`, `1290`), and a loader that starts from that type
/// inherits it: 667 (`Load_667.cpp:80`). STM and STX also start from it
/// (`Load_stm.cpp:232,470`). The FTM loader sets the rule itself
/// (`Load_ftm.cpp:113`).
///
/// A S3M file made by a tracker that is not Scream Tracker clears the rule
/// (`Load_s3m.cpp:431`). Zero-volume is correct for those files too, so the
/// table stays on the safe side.
const ST3_RULE_TYPES: [&str; 5] = ["s3m", "stm", "stx", "667", "ftm"];

/// Returns the silencing method for one module type.
///
/// # Errors
///
/// Returns [`StemError::MissingModuleType`] for an empty value. A module that
/// reports no type cannot be shown to keep its effects on a silenced channel.
pub fn silencing_for(module_type: &str) -> Result<Silencing, StemError> {
    if module_type.trim().is_empty() {
        return Err(StemError::MissingModuleType);
    }
    let lower = module_type.to_ascii_lowercase();
    if ST3_RULE_TYPES.contains(&lower.as_str()) {
        return Ok(Silencing::ZeroVolume);
    }
    Ok(Silencing::ChannelMute)
}
