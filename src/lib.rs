//! The mod2stems library.
//!
//! The command in `src/main.rs` is a thin front end over this crate.

#![forbid(unsafe_code)]

pub mod profile;

pub use profile::AmigaModel;
pub use profile::DEFAULT_MAX_RENDER_SECONDS;
pub use profile::DEFAULT_OUTPUT_RATE;
pub use profile::Profile;
pub use profile::ProfileConfig;
pub use profile::ProfileError;
pub use profile::ProfileRequest;
pub use profile::ResolvedSettings;
pub use profile::WINDOWED_SINC_FILTER_LENGTH;
