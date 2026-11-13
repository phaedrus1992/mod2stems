//! The error type of the isolation renders.

use std::error::Error;
use std::fmt;

use crate::profile::ProfileError;

/// One failed isolation render.
#[derive(Debug, Clone, PartialEq)]
pub enum StemError {
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
    /// The reference render holds no samples.
    EmptyReference {
        /// The type token of the module.
        module_type: String,
        /// The number of channels in the module.
        channels: i32,
    },
    /// The stem and the reference render differ in length.
    StemLengthMismatch {
        /// The channel that the caller named.
        exported: i32,
        /// The number of samples in the stem.
        stem: usize,
        /// The number of samples in the reference render.
        reference: usize,
    },
    /// The channel lies outside the module's channel range.
    ChannelOutOfRange {
        /// The channel that the caller named.
        channel: i32,
        /// The number of channels in the module.
        channels: i32,
    },
    /// The module did not keep a value that the playback policy set.
    ///
    /// The policy reads each value back, because a build can accept a setter
    /// and keep another value. A mismatch means the render would not play what
    /// the policy names (`docs/design.md` §4.2).
    PolicyMismatch {
        /// The policy setting that did not take effect.
        setting: &'static str,
        /// The value that the policy set.
        wanted: i32,
        /// The value that the module reads back.
        actual: i32,
    },
    /// No completed render fixes the common timeline.
    ///
    /// An export with no render has no length to align to, so every stem of it
    /// would be empty (`docs/design.md` §4.2).
    EmptyTimeline,
    /// A render holds a half frame.
    ///
    /// The samples are interleaved stereo, so an odd count cannot name a whole
    /// frame and the timeline or the downmix would split a pair.
    NotStereo {
        /// The number of samples that the render holds.
        samples: usize,
    },
    /// A render is longer than the common timeline.
    ///
    /// Padding cannot shorten a render, and a shorter common end would cut
    /// audio that the library played (`docs/design.md` §4.2).
    RenderPastTimeline {
        /// The number of frames in the render.
        frames: usize,
        /// The number of frames in the common timeline.
        timeline: usize,
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
            Self::UncertainRules { module_type } => write!(
                out,
                "the module type {module_type:?} does not select one silencing method. One token of that type can play with ScreamTracker 3 rules and another token can play without them, so the export cannot prove that a silenced channel keeps its effects. Export a module of a type that the table lists, such as s3m, mod, it, or xm"
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
            Self::EmptyReference {
                module_type,
                channels,
            } => write!(
                out,
                "the module of type {module_type:?} with {channels} channels renders no samples, so every stem would be empty. Choose a subsong that holds orders, or check the module for a song that ends before its first row"
            ),
            Self::StemLengthMismatch {
                exported,
                stem,
                reference,
            } => write!(
                out,
                "the stem of channel {exported} holds {stem} samples and the reference render holds {reference}. The stem must span the same timeline as the reference. Check the module for a song end that depends on the silenced channel"
            ),
            Self::ChannelOutOfRange { channel, channels } => write!(
                out,
                "channel {channel} is not a channel of this module, which has {channels}. Choose a channel in 0..{channels}"
            ),
            Self::PolicyMismatch {
                setting,
                wanted,
                actual,
            } => write!(
                out,
                "the playback policy sets {setting} to {wanted} and the module reads back {actual}. The policy did not take effect on this module. Use a libopenmpt build that offers the value, and check that the module holds the subsong"
            ),
            Self::EmptyTimeline => write!(
                out,
                "the export has no completed render, so no common end aligns the tracks. Render the module once before a stem, and check that the module plays at least one row"
            ),
            Self::NotStereo { samples } => write!(
                out,
                "a render holds {samples} samples, which is not a whole number of stereo frames. The renderer writes interleaved stereo, so an odd count means a truncated read. Render the module again with an even sample count"
            ),
            Self::RenderPastTimeline { frames, timeline } => write!(
                out,
                "a render holds {frames} frames and the common timeline holds {timeline}. The common end is the longest completed render, so a longer render means the timeline was built from another set of renders. Build the timeline from every render that the export writes"
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
