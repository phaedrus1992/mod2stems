//! The error type of the profile resolver.

use std::error::Error;
use std::fmt;

/// One failed profile or rate resolution, or one failed profile apply.
///
/// The resolver fails before any render starts, so every variant states the
/// input that the caller gave and the fix that the caller can apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileError {
    /// The text names no profile.
    UnknownName {
        /// The text that the caller gave.
        name: String,
    },
    /// The `amiga` profile needs an Amiga model, and the caller named none.
    MissingAmigaModel,
    /// The caller named an Amiga model for a profile that does not use one.
    UnexpectedAmigaModel {
        /// The profile that the caller named.
        profile: &'static str,
        /// The model that the caller named.
        model: &'static str,
    },
    /// The output rate lies outside the range that the renderer accepts.
    RateOutOfRange {
        /// The rate that the caller named.
        rate: i32,
        /// The lowest accepted rate.
        lowest: i32,
        /// The highest accepted rate.
        highest: i32,
    },
    /// The module does not offer a control that the profile needs.
    MissingControl {
        /// The control that the profile needs.
        key: &'static str,
    },
    /// The module reads back a value that differs from the applied value.
    SettingMismatch {
        /// The setting that did not take effect.
        setting: &'static str,
        /// The value that the profile set.
        wanted: i32,
        /// The value that the module reads back.
        actual: i32,
    },
    /// The module reads back text that differs from the applied text.
    SettingTextMismatch {
        /// The setting that did not take effect.
        setting: &'static str,
        /// The text that the profile set.
        wanted: &'static str,
        /// The text that the module reads back.
        actual: String,
    },
    /// The two renders that the Amiga check compares have different lengths.
    AmigaComparisonLength {
        /// The length of the plain render, in samples.
        plain: usize,
        /// The length of the emulated render, in samples.
        emulated: usize,
    },
    /// The `amiga` profile does not apply to this module.
    AmigaDoesNotApply,
    /// The `clean` profile cannot render this module as named.
    CleanDoesNotApply {
        /// The module type that the module reports.
        module_type: String,
    },
    /// The two plain renders differ, so the module reads random state.
    Nondeterministic {
        /// The sample index of the first difference.
        sample: usize,
    },
    /// The render reached the configured maximum duration.
    TooLong {
        /// The maximum duration, in seconds.
        limit_seconds: u64,
    },
    /// The library rejected a control value, or read back a different value.
    Library {
        /// The operation that failed.
        operation: &'static str,
        /// The text of the library error, with its input, code, hint, and log.
        message: String,
    },
}

impl ProfileError {
    /// Wraps a wrapper error with the operation that the profile code ran.
    ///
    /// The wrapper names the failing call in `op`. That name stays in the
    /// message, because a library failure can come from a call that the
    /// profile code did not name.
    #[must_use]
    pub fn library(error: &openmpt_ext::OpenmptError, operation: &'static str) -> Self {
        Self::Library {
            operation,
            message: format!(
                "{:?} reports {} (input {:?}, code {:?}, hint {:?}, log {:?})",
                error.op, error.message, error.input, error.code, error.hint, error.log
            ),
        }
    }
}

impl fmt::Display for ProfileError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownName { name } => write!(
                out,
                "profile {name:?} is not a profile. Use one of compatible, amiga, or clean"
            ),
            Self::MissingAmigaModel => write!(
                out,
                "the amiga profile needs a filter model. Choose a500, a1200, or unfiltered"
            ),
            Self::UnexpectedAmigaModel { profile, model } => write!(
                out,
                "the {profile} profile does not use an Amiga model, and the request gives {model}. Remove the model, or choose the amiga profile"
            ),
            Self::RateOutOfRange {
                rate,
                lowest,
                highest,
            } => write!(
                out,
                "the output rate {rate} Hz is outside {lowest}..={highest} Hz. Choose a rate in that range"
            ),
            Self::MissingControl { key } => write!(
                out,
                "this libopenmpt build has no {key:?} control, and the profile needs it. Rebuild against a libopenmpt that offers the control, or choose another profile"
            ),
            Self::SettingMismatch {
                setting,
                wanted,
                actual,
            } => write!(
                out,
                "the profile sets the {setting} to {wanted} and the module reads back {actual}. The profile did not take effect. Use a libopenmpt build that offers the value"
            ),
            Self::SettingTextMismatch {
                setting,
                wanted,
                actual,
            } => write!(
                out,
                "the profile sets {setting} to {wanted:?} and the module reads back {actual:?}. The profile did not take effect. Use a libopenmpt build that offers the value"
            ),
            Self::AmigaComparisonLength { plain, emulated } => write!(
                out,
                "the plain render holds {plain} samples and the emulated render holds {emulated}. Both renders must use the same rate and the same length before their samples can be compared"
            ),
            Self::AmigaDoesNotApply => write!(
                out,
                "the amiga profile does not apply to this module. The emulated render matches the plain render, so the module does not use the Amiga resampler. Choose the compatible or clean profile"
            ),
            Self::CleanDoesNotApply { module_type } => write!(
                out,
                "the clean profile cannot render this module as named. The module type is {module_type:?}, and a drum in that format can keep the nearest resampling mode, so the render would not use windowed-sinc. Choose the compatible or amiga profile"
            ),
            Self::Nondeterministic { sample } => write!(
                out,
                "two renders with the same settings differ at sample {sample}, so the module reads random state. Choose the compatible or clean profile, because the amiga check cannot tell the profile from that noise"
            ),
            Self::TooLong { limit_seconds } => write!(
                out,
                "the render reached the maximum duration of {limit_seconds} s, and the export is incomplete. Raise the maximum duration, or use a shorter module"
            ),
            Self::Library { operation, message } => {
                write!(out, "{operation} failed. libopenmpt reports: {message}")
            }
        }
    }
}

impl Error for ProfileError {}
