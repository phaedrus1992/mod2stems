//! The error type of the profile resolver.

use std::error::Error;
use std::fmt;

/// One failed profile or rate resolution.
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
    /// The caller named two settings that the library cannot apply together.
    ConflictingSettings {
        /// The first setting.
        first: &'static str,
        /// The second setting.
        second: &'static str,
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
    /// The two renders that the Amiga check compares have different lengths.
    AmigaComparisonLength {
        /// The length of the plain render, in samples.
        plain: usize,
        /// The length of the emulated render, in samples.
        emulated: usize,
    },
    /// The `amiga` profile does not apply to this module.
    AmigaDoesNotApply,
    /// The library rejected a control value.
    Library {
        /// The operation that failed.
        operation: &'static str,
        /// The text of the library error.
        message: String,
    },
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
            Self::ConflictingSettings { first, second } => write!(
                out,
                "{first} and {second} cannot be set together, because the library applies one and ignores the other. Remove one of them"
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
            Self::AmigaComparisonLength { plain, emulated } => write!(
                out,
                "the plain render holds {plain} samples and the emulated render holds {emulated}. Both renders must use the same rate and the same length before their samples can be compared"
            ),
            Self::AmigaDoesNotApply => write!(
                out,
                "the amiga profile does not apply to this module. The emulated render matches the plain render, so the module does not use the Amiga resampler. Choose the compatible or clean profile"
            ),
            Self::Library { operation, message } => {
                write!(out, "{operation} failed. libopenmpt reports: {message}")
            }
        }
    }
}

impl Error for ProfileError {}
