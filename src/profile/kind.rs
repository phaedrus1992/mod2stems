//! The profile names that the tool accepts.

use crate::profile::ProfileError;

/// One rendering profile.
///
/// The name is the value that `--profile` accepts. The default is
/// `Compatible`, as `docs/design.md` §2.1 states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Profile {
    /// The library's recommended settings and format compatibility behavior.
    #[default]
    Compatible,
    /// The stock Amiga resampler with an A500 or A1200 filter model.
    Amiga,
    /// No Amiga emulation, with the windowed-sinc interpolation.
    Clean,
}

impl Profile {
    /// Returns the CLI spelling of the profile.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Compatible => "compatible",
            Self::Amiga => "amiga",
            Self::Clean => "clean",
        }
    }

    /// Reads a profile from its CLI spelling.
    ///
    /// # Errors
    ///
    /// Returns an error when the text names no profile.
    pub fn parse(text: &str) -> Result<Self, ProfileError> {
        match text {
            "compatible" => Ok(Self::Compatible),
            "amiga" => Ok(Self::Amiga),
            "clean" => Ok(Self::Clean),
            other => Err(ProfileError::UnknownName {
                name: other.to_owned(),
            }),
        }
    }
}
