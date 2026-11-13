//! The error type of the isolation renders.

use std::error::Error;
use std::fmt;

use crate::profile::ProfileError;

/// One failed isolation render.
#[derive(Debug, Clone, PartialEq)]
pub enum StemError {
    /// The module reports no type, so the silencing method cannot be chosen.
    MissingModuleType,
    /// The type token names no rule set that the table can prove.
    ///
    /// One token can select more than one rule set, so the token cannot show
    /// that a silenced channel keeps its effects (`docs/design.md` §4.1).
    UncertainRules {
        /// The type token that the module reports.
        module_type: String,
    },
    /// A silenced channel does not keep its volume at zero.
    ///
    /// An IT or MPTM channel-volume command, a channel-volume slide, or a song
    /// restart raises the value again, and the channel then leaks into the
    /// stem of another channel (`docs/design.md` §4.1).
    VolumeDrifted {
        /// The channel that the render exports.
        exported: i32,
        /// The silenced channel whose volume changed.
        silenced: i32,
        /// The volume that the read reports, in the range 0.0 through 1.0.
        volume: f64,
    },
    /// A silenced channel does not keep its mute flag.
    ///
    /// A song restart clears the flag again, and the channel then leaks into
    /// the stem of another channel (`docs/design.md` §4.1).
    MuteDrifted {
        /// The channel that the render exports.
        exported: i32,
        /// The silenced channel whose mute flag cleared.
        silenced: i32,
    },
    /// A silenced render visits a different song flow than the plain render.
    TraceDiverged {
        /// The channel that the render exports.
        exported: i32,
        /// The index in the trace where the two flows differ.
        index: usize,
    },
    /// The reference render came from a different module.
    ///
    /// The flow check compares two traces, so a trace from another module or
    /// another type would compare two unrelated songs and pass a stem that it
    /// must reject (`docs/design.md` §4.1).
    ReferenceMismatch {
        /// The channel that the caller named.
        channel: i32,
        /// The number of channels in the module that the render reads.
        channels: i32,
        /// The number of channels in the module of the reference render.
        reference_channels: i32,
        /// The type token of the module that the render reads.
        module_type: String,
        /// The type token of the module of the reference render.
        reference_type: String,
        /// The profile and rate that the export uses.
        profile: String,
        /// The profile and rate that the reference uses.
        reference_profile: String,
    },
    /// The channel lies outside the module's channel range.
    ChannelOutOfRange {
        /// The channel that the caller named.
        channel: i32,
        /// The number of channels in the module.
        channels: i32,
    },
    /// The render or one of its library calls failed.
    Profile(ProfileError),
}

impl From<ProfileError> for StemError {
    fn from(error: ProfileError) -> Self {
        Self::Profile(error)
    }
}

impl fmt::Display for StemError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingModuleType => write!(
                out,
                "the module reports no type, so the export cannot choose a silencing method. The export needs the \"type\" metadata key to decide, so check that libopenmpt reads the file as a tracker module"
            ),
            Self::UncertainRules { module_type } => write!(
                out,
                "the module type {module_type:?} does not select one silencing method. One token of that type can play with ScreamTracker 3 rules and an other token can play without them, so the export cannot prove that a silenced channel keeps its effects. Export a module of a type that the table lists, such as s3m, mod, it, or xm"
            ),
            Self::VolumeDrifted {
                exported,
                silenced,
                volume,
            } => write!(
                out,
                "channel {silenced} reads volume {volume} after the render of channel {exported}, and a silenced channel must stay at zero. A channel-volume command or a song restart raised it, so the stem of channel {exported} holds audio from channel {silenced}. Use a module that does not change channel volume"
            ),
            Self::MuteDrifted { exported, silenced } => write!(
                out,
                "channel {silenced} reads as unmuted after the render of channel {exported}, and a silenced channel must stay muted. A song restart cleared the flag, so the stem of channel {exported} holds audio from channel {silenced}. Use a module that does not restart the song"
            ),
            Self::TraceDiverged { exported, index } => write!(
                out,
                "the render of channel {exported} visits a different song flow than the plain render, at position {index} of the trace. The silencing changed the song flow, so the stem is not a faithful split. Check the module for a speed, tempo, pattern jump, or pattern loop command on channel {exported}"
            ),
            Self::ReferenceMismatch {
                channel,
                channels,
                reference_channels,
                module_type,
                reference_type,
                profile,
                reference_profile,
            } => write!(
                out,
                "the reference render does not match channel {channel}. The module reads type {module_type:?} with {channels} channels under {profile}, and the reference reads type {reference_type:?} with {reference_channels} channels under {reference_profile}. Render the reference from the same module bytes and the same profile, then export the channel"
            ),
            Self::ChannelOutOfRange { channel, channels } => write!(
                out,
                "channel {channel} is not a channel of this module, which has {channels}. Choose a channel in 0..{channels}"
            ),
            Self::Profile(error) => write!(out, "{error}"),
        }
    }
}

impl Error for StemError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Profile(error) => Some(error),
            _ => None,
        }
    }
}
