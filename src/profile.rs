//! Rendering profiles and output rate resolution.
//!
//! A profile is a named set of playback settings. A request resolves it once,
//! and every stem and reference render shares the result. Design: `docs/design.md`
//! §2, and `docs/design/pre-implementation-validation.md` §6 and §7.

pub mod amiga;
pub mod apply;
pub mod config;
pub mod error;
pub mod kind;
pub mod model;
pub mod request;
pub mod settings;

pub use amiga::AmigaVerdict;
pub use amiga::verify_amiga_applies;
pub use config::ProfileConfig;
pub use error::ProfileError;
pub use kind::Profile;
pub use model::AmigaModel;
pub use request::ProfileRequest;
pub use settings::ResolvedSettings;

/// The output rate that the tool uses when the caller names none.
///
/// The libopenmpt header recommends 48000 Hz unless the user asks for another
/// rate (`libopenmpt/libopenmpt.h`).
pub const DEFAULT_OUTPUT_RATE: i32 = 48_000;

/// The interpolation filter length that selects windowed-sinc resampling.
///
/// The library maps a length of 8 or more onto its sinc resampler, and 0 onto
/// the same mode. The named length is explicit, so the resolved config records
/// a value that is not the "unset" of 0.
pub const WINDOWED_SINC_FILTER_LENGTH: i32 = 8;

/// The volume ramping value that keeps the library default.
///
/// A value of -1 selects the library's recommended ramping strength.
pub const LIBRARY_DEFAULT_VOLUME_RAMPING: i32 = -1;
