//! The silencing method that one module type needs.
//!
//! A per-channel render must silence every other channel and still keep the
//! timing and global effects that those channels carry. The stock API has two
//! methods, and neither covers every type (`docs/design.md` §4.1).
//!
//! The library does not expose the playback rules that decide this. The only
//! fact that the C API gives is the module type token, so the token selects the
//! method. A token that can select more than one rule set fails, because a
//! wrong method gives a stem with missing effects and no check can see it.
//! Design: `docs/design.md` §4.1.

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

/// The module types that play with `ScreamTracker` 3 rules.
///
/// `GetDefaultPlaybackBehaviour(MOD_TYPE_S3M)` sets `kST3NoMutedChannels`
/// (`Sndfile.cpp:1290`). `InitializeGlobals` applies the rules of the save
/// format, and `GetBestSaveFormat` maps 669, FAR, STM, and MTM to the S3M type
/// (`Sndfile.cpp:1433`). The loaders of 667, C67, CBA, ETX, and FMT start from
/// the S3M type itself. The FTM loader sets the rule on a MOD type
/// (`Load_ftm.cpp:113`).
///
/// A S3M file that a tracker other than Scream Tracker made clears the rule
/// (`Load_s3m.cpp:431`). Zero channel volume is correct for those files too, so
/// the table stays on the safe side.
const ST3_RULE_TYPES: [&str; 12] = [
    "667", "669", "c67", "cba", "etx", "far", "fmt", "ftm", "mtm", "s3m", "stm", "stx",
];

/// The module types that play with no `ScreamTracker` 3 rule.
///
/// Each of these tokens pairs with one save format, and that format never sets
/// the rule. A token that is not in either list fails, so a new loader cannot
/// select the unsafe method by default.
const OTHER_RULE_TYPES: [&str; 43] = [
    "ams", "dbm", "digi", "dsym", "dtm", "fc", "gmc", "gt2", "gtk", "ice", "imf", "ims", "it",
    "itp", "kris", "mdl", "med", "mid", "mod", "mptm", "mt2", "mus", "okt", "oxm", "plm", "ptm",
    "puma", "rmi", "rtm", "sfx", "sfx2", "smod", "st26", "stk", "stp", "symmod", "tcb", "uax",
    "ult", "unic", "wav", "xm", "xmf",
];

/// Returns the silencing method for one module type.
///
/// The type token chooses the playback rules when the loader runs, so the token
/// decides the method. Six tokens are left out on purpose, because one token
/// names more than one rule set:
///
/// 1. `amf` comes from the AMF loader, which starts as AMF0 or as AMF, and
///    those two types take different rules.
/// 2. `dmf` comes from the AMF loader and from the DMF loader, and those two
///    loaders start from different types.
/// 3. `dsm` comes from two DSM loaders, one of which starts as MOD.
/// 4. `gdm` takes its type from a table inside the loader.
/// 5. `mo3` is a container, so the type depends on the file it holds.
/// 6. `psm` maps to S3M only when the module has 16 channels or fewer, a
///    single sequence, and a plain channel setup (`Sndfile.cpp:1419`).
///
/// # Errors
///
/// Returns [`StemError::MissingModuleType`] for an empty value, because a module
/// that reports no type cannot be shown to keep its effects on a silenced
/// channel. Returns [`StemError::UncertainRules`] for a token that is not in
/// either table.
pub fn silencing_for(module_type: &str) -> Result<Silencing, StemError> {
    let token = module_type.trim();
    if token.is_empty() {
        return Err(StemError::MissingModuleType);
    }
    let lower = token.to_ascii_lowercase();
    if ST3_RULE_TYPES.contains(&lower.as_str()) {
        return Ok(Silencing::ZeroVolume);
    }
    if OTHER_RULE_TYPES.contains(&lower.as_str()) {
        return Ok(Silencing::ChannelMute);
    }
    Err(StemError::UncertainRules {
        module_type: token.to_owned(),
    })
}
